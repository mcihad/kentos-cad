"""Python completion from AST, installed imports and the live namespace.

Source is parsed, never executed for completion. Member lookup uses static
inspection so properties and arbitrary expressions are not evaluated.
"""
import builtins
import importlib
import inspect
import keyword
import pkgutil
import re
import types
from functools import lru_cache


class Symbol:
    def __init__(self, name, kind, detail="", doc="", members=None, parameters=None):
        self.name, self.kind, self.detail, self.doc = name, kind, detail, doc
        self.members = members or {}
        self.parameters = parameters or []


def static_members(value):
    if isinstance(value, Symbol):
        return value.members
    result = {}
    if isinstance(value, (types.ModuleType, type)):
        result.update(vars(value))
    else:
        dictionary = inspect.getattr_static(value, "__dict__", {})
        if isinstance(dictionary, dict):
            result.update(dictionary)
    for base in (value.__mro__ if isinstance(value, type) else type(value).__mro__):
        result.update({name: member for name, member in vars(base).items() if name not in result})
    return result


def resolve(expression, scope):
    parts = expression.split(".")
    if not all(part.isidentifier() for part in parts):
        return None
    value = scope.get(parts[0])
    for name in parts[1:]:
        value = static_members(value).get(name) if value is not None else None
    return value


def info(name, value, member=False):
    if isinstance(value, Symbol):
        return value.kind, value.detail, value.doc
    if isinstance(value, (staticmethod, classmethod)):
        value = value.__func__
    if isinstance(value, types.ModuleType):
        kind = "package" if "__path__" in vars(value) else "module"
        detail = value.__name__
    elif isinstance(value, type):
        kind, detail = "class", name
    elif isinstance(value, property):
        kind, detail = "property", name
    elif inspect.isroutine(value) or isinstance(value, (types.MethodDescriptorType, types.WrapperDescriptorType)):
        kind, detail = ("method" if member else "function"), name
    else:
        kind, detail = ("constant" if name.isupper() else "variable"), f"{name}: {type(value).__name__}"
    if kind in ("class", "function", "method"):
        try:
            detail = name + str(inspect.signature(value))
        except (ValueError, TypeError):
            pass
    doc = inspect.getattr_static(value, "__doc__", "") if value is not None else ""
    if not isinstance(doc, str):
        doc = ""
    return kind, detail[:240], next((line.strip() for line in doc.splitlines() if line.strip()), "")[:220]


def function(node, method=False):
    arguments = ast.unparse(node.args)
    annotation = " -> " + ast.unparse(node.returns) if node.returns else ""
    names = [arg.arg for arg in node.args.posonlyargs + node.args.args + node.args.kwonlyargs if arg.arg not in ("self", "cls")]
    return Symbol(node.name, "method" if method else "function", f"{node.name}({arguments}){annotation}",
                  (ast.get_docstring(node) or "").strip().split("\n")[0][:220], parameters=names)


def inferred(node, scope):
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.List):
        return []
    if isinstance(node, (ast.Set, ast.SetComp)):
        return set()
    if isinstance(node, (ast.Dict, ast.DictComp)):
        return {}
    if isinstance(node, ast.Tuple):
        return ()
    if isinstance(node, ast.ListComp):
        return []
    if isinstance(node, (ast.Name, ast.Attribute)):
        return resolve(ast.unparse(node), scope)
    if isinstance(node, ast.Call):
        value = resolve(ast.unparse(node.func), scope)
        if isinstance(value, type) or isinstance(value, Symbol) and value.kind == "class":
            return Symbol("", "variable", getattr(value, "__name__", getattr(value, "name", "")), members=static_members(value))
    return Symbol("", "variable", "Yerel değişken")


def parsed_source(source, row):
    try:
        return ast.parse(source)
    except SyntaxError:
        lines = source.splitlines()
        if row < len(lines):
            indent = re.match(r"\s*", lines[row]).group()
            lines[row] = indent + "pass"
        while lines:
            try:
                return ast.parse("\n".join(lines))
            except SyntaxError:
                lines.pop()
        return ast.parse("")


def source_scope(source, row, runtime):
    scope = dict(vars(builtins))
    scope.update(runtime)
    tree = parsed_source(source, row)

    def visit(nodes, destination, member=False):
        for node in nodes:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                symbol = function(node, member)
                if any(isinstance(decorator, ast.Name) and decorator.id == "property" for decorator in node.decorator_list):
                    symbol.kind = "property"
                destination[node.name] = symbol
                if member:
                    for statement in ast.walk(node):
                        if isinstance(statement, (ast.Assign, ast.AnnAssign)):
                            targets = statement.targets if isinstance(statement, ast.Assign) else [statement.target]
                            for target in targets:
                                if isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name) and target.value.id == "self":
                                    destination[target.attr] = Symbol(target.attr, "property", target.attr,
                                        "Sınıfın örnek alanı.", members=static_members(inferred(statement.value, scope)))
                if node.lineno <= row + 1 <= node.end_lineno and not member:
                    for arg in node.args.posonlyargs + node.args.args + node.args.kwonlyargs:
                        inferred_type = inferred(arg.annotation, destination) if arg.annotation else None
                        destination[arg.arg] = Symbol(arg.arg, "parameter", arg.arg + (": " + ast.unparse(arg.annotation) if arg.annotation else ""), members=static_members(inferred_type) if inferred_type is not None else {})
                    visit(node.body, destination)
            elif isinstance(node, ast.ClassDef):
                members = {}
                for base in node.bases:
                    value = resolve(ast.unparse(base), scope)
                    if value is not None:
                        members.update(static_members(value))
                visit(node.body, members, True)
                destination[node.name] = Symbol(node.name, "class", node.name,
                    (ast.get_docstring(node) or "").strip().split("\n")[0][:220], members)
                if node.lineno <= row + 1 <= node.end_lineno:
                    destination.update(members)
                    destination["self"] = destination[node.name]
            elif isinstance(node, (ast.Assign, ast.AnnAssign)):
                targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                value = inferred(node.value, destination) if node.value else inferred(node.annotation, destination)
                for target in targets:
                    if isinstance(target, ast.Name):
                        destination[target.id] = value
                    elif isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name) and target.value.id == "self":
                        destination[target.attr] = value
            elif isinstance(node, (ast.Import, ast.ImportFrom)):
                try:
                    for alias in node.names:
                        if isinstance(node, ast.Import):
                            destination[alias.asname or alias.name.split(".")[0]] = importlib.import_module(alias.name if alias.asname else alias.name.split(".")[0])
                        elif node.module and not node.level:
                            value = importlib.import_module(node.module)
                            if alias.name == "*":
                                destination.update({name: value for name, value in vars(value).items() if not name.startswith("_")})
                            else:
                                destination[alias.asname or alias.name] = static_members(value).get(alias.name)
                except (ImportError, ValueError, AttributeError):
                    pass
            elif isinstance(node, (ast.For, ast.AsyncFor)):
                if isinstance(node.target, ast.Name):
                    destination[node.target.id] = Symbol(node.target.id, "variable", "Döngü değişkeni")
                visit(node.body, destination, member)
            elif isinstance(node, (ast.If, ast.While, ast.With, ast.Try)):
                visit(node.body, destination, member)
                visit(getattr(node, "orelse", []), destination, member)
    visit(tree.body, scope)
    return scope


@lru_cache(maxsize=24)
def module_names(package=""):
    if not package:
        names = {name: False for name in sys.builtin_module_names + tuple(sys.stdlib_module_names)}
        names.update({item.name: item.ispkg for item in pkgutil.iter_modules()})
        return names
    try:
        module = importlib.import_module(package)
        if hasattr(module, "__path__"):
            return {item.name: item.ispkg for item in pkgutil.iter_modules(module.__path__)}
        return {name: "__path__" in vars(value) for name, value in vars(module).items() if isinstance(value, types.ModuleType)}
    except (ImportError, ValueError, AttributeError):
        return {}


def complete(source, row, column, runtime):
    lines = source.split("\n")
    before = lines[row].encode()[:column].decode() if row < len(lines) else ""
    prefix = re.search(r"[\w]*$", before).group()
    records = {}

    def add(name, value=None, kind=None, insert=None, detail=None):
        if not name.startswith(prefix) or (name.startswith("_") and not prefix.startswith("_")):
            return
        detected, description, doc = info(name, value)
        records[name] = (kind or detected, name, insert or name, detail or description, doc)

    stripped = before.lstrip()
    if re.match(r"(?:import\s+|from\s+)[\w.,\s]*$", stripped) and " import " not in stripped:
        expression = re.split(r"[\s,]+", stripped)[-1]
        package = expression.rsplit(".", 1)[0] if "." in expression else ""
        for name, is_package in module_names(package).items():
            add(name, kind="package" if is_package else "module", detail=(package + "." if package else "") + name)
    elif (match := re.match(r"from\s+([\w.]+)\s+import\s+", stripped)):
        try:
            module = importlib.import_module(match.group(1))
            for name, value in static_members(module).items():
                add(name, value)
            for name, is_package in module_names(match.group(1)).items():
                add(name, kind="package" if is_package else "module")
        except (ImportError, ValueError):
            pass
    elif " as " not in stripped:
        scope = source_scope(source, row, runtime)
        expression = re.search(r"[\w.]*$", before).group()
        if "." in expression:
            value = resolve(expression.rsplit(".", 1)[0], scope)
            if value is not None:
                for name, member in static_members(value).items():
                    kind, detail, doc = info(name, member, not isinstance(value, types.ModuleType))
                    add(name, member, kind=kind, detail=detail)
        else:
            for name, value in scope.items():
                add(name, value)
            for name in keyword.kwlist + keyword.softkwlist:
                add(name, kind="keyword", detail="Python anahtar sözcüğü")
            calls = list(re.finditer(r"([\w.]+)\(([^()]*)$", before))
            if calls:
                call = resolve(calls[-1].group(1), scope)
                names = call.parameters if isinstance(call, Symbol) else []
                if not names and call is not None:
                    try:
                        names = [name for name, parameter in inspect.signature(call).parameters.items()
                                 if parameter.kind in (parameter.POSITIONAL_OR_KEYWORD, parameter.KEYWORD_ONLY)]
                    except (TypeError, ValueError):
                        pass
                supplied = set(re.findall(r"(\w+)\s*=", calls[-1].group(2)))
                for name in names:
                    if name not in supplied:
                        add(name, kind="parameter", insert=name + "=", detail=info(calls[-1].group(1), call)[1])
    return sorted(records.values(), key=lambda item: (item[1].startswith("_"), item[0] != "parameter", item[1]))[:128]


def completion_response(payload, runtime):
    location, source = payload.split("\n", 1)
    row, column = map(int, location.split(","))
    try:
        items = complete(source, row, column, runtime)
        return "\n".join(kind + "\t" + "\t".join(word.encode().hex() for word in (name, insertion, detail, doc))
                         for kind, name, insertion, detail, doc in items)
    except Exception:
        return ""
