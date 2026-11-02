"""ADR 0837: independently encoded circular path-edge constraint files (no native KentOS writer)."""
from copy import deepcopy

def files(document, container, load):
    d=deepcopy(load('arc-edge-constraints.json'))
    payload=document(d)
    assert payload.count(b'\x67version\x18\x9d')==1, 'circular path constraints require schema 157'
    out={'arc-edge-constraints.kcad':container(payload)}
    # Both the last supported predecessor and the concurrently reserved schema reject the new feature.
    for version in (155,156):
        out[f'broken/arc-edge-in-schema-{version}.kcad']=container(payload.replace(b'\x67version\x18\x9d',b'\x67version\x18'+bytes([version])))
    def broken(name, change):
        one=deepcopy(d);change(one['constraints']['constraints'][2]['rule']['a']['element'])
        out[f'broken/arc-edge-{name}.kcad']=container(document(one))
    broken('extra-field',lambda e:e.update(guess=True))
    broken('index-too-large',lambda e:e.update(index=10_000_001))
    broken('radius-on-straight-tag',lambda e:e.update(kind='segment'))
    return out
