from typing import Protocol

from pydantic import BaseModel


class Product(BaseModel):
    sku: str
    name: str
    price: float
    url: str


class Category(BaseModel):
    id: str
    name: str
    url: str


class WebstoreParser(Protocol):
    async def get_products(self, category: Category) -> list[Product]: ...
    async def get_categories(self, url: str) -> list[Category]: ...
    async def get_pages(self, url: str) -> list[str]: ...


class HTTPGet(Protocol):
    async def get(self, url: str) -> tuple[int, str]: ...
