from dataclasses import dataclass
from typing import Protocol

from pydantic import BaseModel


class UserPayload(BaseModel):
    user_id: str
    email: str
    timestamp: str


@dataclass
class UserFilter:
    term: str
    limit: int


class Identifiable(Protocol):
    def identity(self) -> str: ...


def normalize_email(value: str) -> str:
    return value.strip().lower()
