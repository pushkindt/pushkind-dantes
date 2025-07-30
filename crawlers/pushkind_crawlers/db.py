import datetime as dt

from typing import Iterable

from pushkind_crawlers.crawler.protocols import Category
from pushkind_crawlers.crawler.protocols import Product as ParsedProduct
from sqlalchemy import (
    Column,
    Float,
    Integer,
    String,
    create_engine,
    func,
    select,
    delete,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, Session, mapped_column
from sqlalchemy.types import TIMESTAMP

from sentence_transformers import SentenceTransformer
import faiss
import numpy as np


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
    num_products: Mapped[int] = mapped_column(Integer, nullable=False, default=0)


class Product(Base):
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


class Benchmark(Base):
    __tablename__ = "benchmarks"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    hub_id: Mapped[int] = mapped_column(Integer, nullable=False)
    name: Mapped[str] = mapped_column(String, nullable=False)
    sku: Mapped[str] = mapped_column(String, nullable=False)
    category: Mapped[str] = mapped_column(String, nullable=False)
    units: Mapped[str] = mapped_column(String, nullable=False)
    price: Mapped[float] = mapped_column(Float, nullable=False)
    amount: Mapped[float] = mapped_column(Float, nullable=False)
    description: Mapped[str] = mapped_column(String, nullable=False)
    created_at = Column(TIMESTAMP, nullable=False, server_default=func.now())
    updated_at = Column(TIMESTAMP, nullable=False, server_default=func.now())


class ProductBenchmark(Base):
    __tablename__ = "product_benchmark"
    product_id: Mapped[int] = mapped_column(Integer, primary_key=True)
    benchmark_id: Mapped[int] = mapped_column(Integer, primary_key=True)


def save_products(
    db_url: str,
    crawler_selector: str,
    products: list[ParsedProduct],
) -> None:
    """Persist parsed products to the database."""
    engine = create_engine(db_url)
    with Session(engine) as session:
        crawler = session.scalars(select(Crawler).where(Crawler.selector == crawler_selector)).one()
        session.query(Product).filter(Product.crawler_id == crawler.id).delete()
        for product in products:
            session.add(
                Product(
                    crawler_id=crawler.id,
                    name=product.name,
                    sku=product.sku,
                    category=product.category,
                    units=product.units,
                    price=product.price,
                    amount=product.amount,
                    description=product.description,
                    url=product.url,
                )
            )
        session.query(Crawler).filter(Crawler.selector == crawler_selector).update(
            {
                "num_products": len(products),
                "processing": False,
                "updated_at": dt.datetime.now(),
            }
        )
        session.commit()


def _prompt(name: str, category: str | None, description: str | None, units: str | None, amount: float | None) -> str:
    return (
        f"\u041d\u0430\u0437\u0432\u0430\u043d\u0438\u0435: {name}\n"
        f"\u041a\u0430\u0442\u0435\u0433\u043e\u0440\u0438\u044f: {category or ''}\n"
        f"\u041e\u043f\u0438\u0441\u0430\u043d\u0438\u0435: {description or ''}\n"
        f"\u0415\u0434\u0438\u043d\u0438\u0446\u044b: {units or ''}\n"
        f"\u041e\u0431\u044a\u0435\u043c \u0443\u043f\u0430\u043a\u043e\u0432\u043a\u0438: {amount or ''}"
    )


def update_benchmark_associations(db_url: str, crawler_selector: str) -> None:
    engine = create_engine(db_url)
    with Session(engine) as session:
        crawler = session.scalars(select(Crawler).where(Crawler.selector == crawler_selector)).one()

        products = session.scalars(select(Product).where(Product.crawler_id == crawler.id)).all()
        if not products:
            return

        product_ids = [p.id for p in products]

        session.execute(
            delete(ProductBenchmark).where(ProductBenchmark.product_id.in_(product_ids))
        )

        benchmarks = session.scalars(select(Benchmark)).all()
        if not benchmarks:
            session.commit()
            return

        model = SentenceTransformer("paraphrase-multilingual-mpnet-base-v2")

        product_texts = [
            _prompt(p.name, p.category, p.description, p.units, p.amount) for p in products
        ]
        benchmark_texts = [
            _prompt(b.name, b.category, b.description, b.units, b.amount) for b in benchmarks
        ]

        prod_emb = model.encode(product_texts, normalize_embeddings=True)
        bench_emb = model.encode(benchmark_texts, normalize_embeddings=True)

        prod_emb = np.array(prod_emb, dtype="float32")
        bench_emb = np.array(bench_emb, dtype="float32")

        index = faiss.IndexFlatIP(prod_emb.shape[1])
        index.add(prod_emb)

        k = min(10, len(products))
        distances, indices = index.search(bench_emb, k)

        for b_idx, prod_idxs in enumerate(indices):
            benchmark_id = benchmarks[b_idx].id
            for p_idx in prod_idxs:
                session.add(
                    ProductBenchmark(
                        product_id=products[p_idx].id,
                        benchmark_id=benchmark_id,
                    )
                )

        session.commit()
