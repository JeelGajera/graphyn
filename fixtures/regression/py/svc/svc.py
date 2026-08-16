from ..models.types import Alpha, Beta


def use_alpha(whatever: Alpha) -> str:
    return whatever.a_field


def use_beta(anything: Beta) -> str:
    return anything.b_field
