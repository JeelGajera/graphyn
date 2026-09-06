from .services import UserService, format_name as fmt
from requests import get


def run():
    # Construction and invocation are spelled identically in Python; the kind
    # is decided from the resolved target, not from the name.
    service = UserService()

    # Call through a renamed import, resolving to the canonical symbol.
    name = fmt("Ada", "Lovelace")

    # Attribute call: recorded as a property access on the receiver, never as
    # a call edge to the class itself.
    service.handle()

    # Neither of these names a symbol this file can resolve, so no edge and no
    # diagnostic — there is nothing here a user could fix.
    print(name)

    # A third-party function: the call is real, but the package is not what
    # ran. The import edge already records the dependency.
    get("/health")

    return len(name)
