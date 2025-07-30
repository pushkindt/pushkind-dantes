import datetime as dt
import logging

import faiss
import numpy as np
from pushkind_crawlers.crawler.protocols import Product as ParsedProduct
from sentence_transformers import SentenceTransformer
from sqlalchemy import (
    Column,
    Float,
    Integer,
    String,
    create_engine,
    delete,
    func,
    select,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, Session, mapped_column
from sqlalchemy.types import TIMESTAMP

log = logging.getLogger(__name__)


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
    distance: Mapped[float] = mapped_column(Float, nullable=False)


def save_products(
    db_url: str,
    crawler_selector: str,
    products: list[ParsedProduct],
) -> None:
    """Persist parsed products to the database."""
    engine = create_engine(db_url)
    with Session(engine) as session:
        crawler = session.scalars(select(Crawler).where(Crawler.selector == crawler_selector)).one()
        old_ids = session.scalars(select(Product.id).where(Product.crawler_id == crawler.id)).all()
        if old_ids:
            session.execute(delete(ProductBenchmark).where(ProductBenchmark.product_id.in_(old_ids)))
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
        f"Название: {name}\n"
        f"Категория: {category or ''}\n"
        f"Описание: {description or ''}\n"
        f"Единицы: {units or ''}\n"
        f"Объём: {amount or ''}"
    )


def update_benchmark_associations(db_url: str, crawler_selector: str) -> None:
    engine = create_engine(db_url)
    with Session(engine) as session:
        crawler = session.scalars(select(Crawler).where(Crawler.selector == crawler_selector)).one()

        products = session.scalars(select(Product).where(Product.crawler_id == crawler.id)).all()
        if not products:
            return

        benchmarks = session.scalars(select(Benchmark)).all()
        if not benchmarks:
            return

        model = SentenceTransformer("sentence-transformers/paraphrase-multilingual-mpnet-base-v2")

        product_texts = [_prompt(p.name, p.category, p.description, p.units, p.amount) for p in products]
        benchmark_texts = [_prompt(b.name, b.category, b.description, b.units, b.amount) for b in benchmarks]

        prod_emb = model.encode(product_texts, normalize_embeddings=True)
        bench_emb = model.encode(benchmark_texts, normalize_embeddings=True)

        prod_emb = np.array(prod_emb, dtype="float32")
        bench_emb = np.array(bench_emb, dtype="float32")

        index = faiss.IndexFlatIP(prod_emb.shape[1])
        index.add(prod_emb)

        k = min(10, len(products))
        distances, indices = index.search(bench_emb, k)

        threshold = 0.75
        for b_idx, (prod_idxs, dist_row) in enumerate(zip(indices, distances)):
            benchmark_id = benchmarks[b_idx].id
            for p_idx, distance in zip(prod_idxs, dist_row):
                if distance >= threshold:
                    session.add(
                        ProductBenchmark(
                            product_id=products[p_idx].id,
                            benchmark_id=benchmark_id,
                            distance=float(distance),
                        )
                    )

        session.commit()
