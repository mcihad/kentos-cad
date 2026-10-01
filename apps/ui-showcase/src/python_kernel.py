"""Persistent interpreter for the UI showcase; a hex framed stdio protocol.

The Rust host owns this process. User output is captured before it reaches the
protocol, and capped while it is written rather than after a huge allocation.
"""

import ast
import codeop
import contextlib
import io
import sys
import time
import traceback

# COMPLETION_PROVIDER


class Output(io.TextIOBase):
    def __init__(self):
        self.parts = []
        self.size = 0
        self.truncated = False

    def writable(self):
        return True

    def write(self, words):
        remaining = max(0, 128 * 1024 - self.size)
        kept = words[:remaining]
        self.parts.append(kept) if kept else None
        self.size += len(kept)
        self.truncated |= len(kept) < len(words)
        return len(words)

    def value(self):
        return "".join(self.parts) + ("\n… çıktı sınırına ulaşıldı" if self.truncated else "")


namespace = {"__name__": "__console__"}
compiler = codeop.CommandCompiler()
wire = sys.stdout


def unavailable_input(*args, **kwargs):
    raise RuntimeError("Bu REPL input() desteklemiyor. Değeri kod içinde tanımlayın.")


namespace["input"] = unavailable_input


def display(value):
    if value is not None:
        namespace["_"] = value
        print(repr(value))


sys.displayhook = display
for request in sys.stdin:
    execution, mode, encoded = request.rstrip("\n").split(" ", 2)
    source = bytes.fromhex(encoded).decode("utf-8")
    if mode == "complete":
        response = completion_response(source, namespace)
        wire.write(f"C {execution} {response.encode().hex()}\n")
        wire.flush()
        continue
    output, errors = Output(), Output()
    success, incomplete = True, False
    started = time.perf_counter_ns()
    filename = "geometry.py" if mode == "script" else "<console>"
    with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
        try:
            compiled = compiler(source, filename, "exec")
            if compiled is None:
                incomplete = True
            elif mode == "script":
                exec(compiled, namespace)
            else:
                # Pasted statements work too; the final expression gets the
                # usual REPL display hook and remains available as `_`.
                tree = ast.parse(source, filename, "exec")
                if tree.body and isinstance(tree.body[-1], ast.Expr):
                    final = tree.body.pop()
                    exec(compile(tree, filename, "exec"), namespace)
                    display(eval(compile(ast.Expression(final.value), filename, "eval"), namespace))
                else:
                    exec(compiled, namespace)
        except BaseException:
            success = False
            traceback.print_exc()
    elapsed = time.perf_counter_ns() - started
    wire.write(f"R {execution} {int(success)} {int(incomplete)} {elapsed} {output.value().encode().hex()} {errors.value().encode().hex()}\n")
    wire.flush()
