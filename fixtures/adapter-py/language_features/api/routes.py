from typing import TYPE_CHECKING

from fastapi import Depends

from ..models import UserPayload
from ..models.user import UserFilter, normalize_email

if TYPE_CHECKING:
    from ..models.order import Order


def describe(payload: UserPayload, filters: UserFilter) -> str:
    return payload.email + payload.user_id + filters.term


def make_route(dependency=Depends(normalize_email)):
    return dependency
