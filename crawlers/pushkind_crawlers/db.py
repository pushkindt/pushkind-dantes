import datetime as dt

from sqlalchemy import (
    Column,
    Float,
    Integer,
    String,
    create_engine,
    func,
    select,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, Session, mapped_column
from sqlalchemy.types import TIMESTAMP

from pushkind_crawlers.crawler.protocols import Category, Product as ParsedProduct


def turn_off_processing(db_url: str, crawler_id: str):
    engine = create_engine(db_url)
    with Session(engine) as session:
        stmt = select(Crawler).where(Crawler.selector == crawler_id)
        crawler = session.scalars(stmt).one()
        crawler.processing = False
        crawler.updated_at = dt.datetime.now()  # type: ignore
        session.commit()


class Base(DeclarativeBase):
    pass


class Crawler(Base):
    __tablename__ = "crawlers"

    id: Mapped[int] = mapped_column(primary_key=True)
    name: Mapped[str] = mapped_column(nullable=False)
    url: Mapped[str] = mapped_column(nullable=False)
    selector: Mapped[str] = mapped_column(nullable=False)
    processing: Mapped[bool] = mapped_column(nullable=False, default=False)
    updated_at = Column(TIMESTAMP, nullable=False, server_default=func.now())


class ProductDB(Base):
    __tablename__ = "products"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    crawler_id: Mapped[int] = mapped_column(Integer, nullable=False)
    name: Mapped[str] = mapped_column(String, nullable=False)
    sku: Mapped[str] = mapped_column(String, nullable=False)
    category: Mapped[str | None] = mapped_column(String, nullable=True)
    units: Mapped[str | None] = mapped_column(String, nullable=True)
    price: Mapped[float] = mapped_column(Float, nullable=False)
    amount: Mapped[float | None] = mapped_column(Float, nullable=True)
    description: Mapped[str | None] = mapped_column(String, nullable=True)
    url: Mapped[str] = mapped_column(String, nullable=False)
    created_at = Column(TIMESTAMP, nullable=False, server_default=func.now())
    updated_at = Column(TIMESTAMP, nullable=False, server_default=func.now())


def save_products(
    db_url: str,
    crawler_selector: str,
    items: list[tuple[Category, list[ParsedProduct]]],
) -> None:
    """Persist parsed products to the database."""
    engine = create_engine(db_url)
    with Session(engine) as session:
        crawler = session.scalars(
            select(Crawler).where(Crawler.selector == crawler_selector)
        ).one()
        for category, products in items:
            for product in products:
                session.add(
                    ProductDB(
                        crawler_id=crawler.id,
                        name=product.name,
                        sku=product.sku,
                        category=category.name,
                        units=None,
                        price=product.price,
                        amount=None,
                        description=None,
                        url=product.url,
                    )
                )
        session.commit()
