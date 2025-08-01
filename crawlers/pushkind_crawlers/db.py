import datetime as dt
import logging

import faiss
import numpy as np
from pushkind_crawlers.crawler.protocols import Product as ParsedProduct
from pushkind_crawlers.embedding import prompt_to_embedding
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
from sqlalchemy.types import BLOB, TIMESTAMP, TypeDecorator

log = logging.getLogger(__name__)


class FloatVectorType(TypeDecorator):
    impl = BLOB
    cache_ok = True

    def process_bind_param(self, value, dialect):
        if value is None:
            return None
        return np.asarray(value, dtype=np.float32).tobytes()

    def process_result_value(self, value, dialect):
        if value is None:
            return None
        return np.frombuffer(value, dtype=np.float32).tolist()


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
    embedding: Mapped[list[float] | None] = mapped_column(FloatVectorType(), nullable=True)


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
    embedding: Mapped[list[float] | None] = mapped_column(FloatVectorType(), nullable=True)
    processing: Mapped[bool] = mapped_column(nullable=False, default=False)


class ProductBenchmark(Base):
    __tablename__ = "product_benchmark"
    product_id: Mapped[int] = mapped_column(Integer, primary_key=True)
    benchmark_id: Mapped[int] = mapped_column(Integer, primary_key=True)
    distance: Mapped[float] = mapped_column(Float, nullable=False)


def set_crawler_status(db_url: str, crawler_selector: str, processing: bool, num_products: int | None = None):
    engine = create_engine(db_url)
    with Session(engine) as session:
        updates = {
            "processing": processing,
            "updated_at": dt.datetime.now(),
        }
        if num_products is not None:
            updates["num_products"] = num_products
        session.query(Crawler).filter(Crawler.selector == crawler_selector).update(updates)  # type: ignore
        session.commit()


def set_benchmark_status(db_url: str, benchmark_id: int, processing: bool):
    engine = create_engine(db_url)
    with Session(engine) as session:
        session.query(Benchmark).filter(Benchmark.id == benchmark_id).update(
            {
                "processing": processing,
            }
        )
        session.commit()


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
                    embedding=None,
                )
            )
        session.commit()


def _prompt(
    name: str,
    sku: str,
    category: str | None,
    units: str | None,
    price: float,
    amount: float | None,
    description: str | None,
) -> str:
    return f"Name: {name}\nSKU: {sku}\nCategory: {category or ''}\nUnits: {units or ''}\nPrice: {price}\nAmount: {amount or ''}\nDescription: {description or ''}"


def update_benchmark_associations(
    db_url: str, crawler_selector: str | None = None, benchmark_id: int | None = None
) -> None:

    if crawler_selector is None and benchmark_id is None:
        raise ValueError("Either crawler_selector or benchmark_id must be provided")

    if crawler_selector is not None and benchmark_id is not None:
        raise ValueError("Only one of crawler_selector or benchmark_id must be provided")

    engine = create_engine(db_url)
    with Session(engine) as session:

        product_query = select(Product)
        if crawler_selector is not None:
            crawler = session.scalars(select(Crawler).where(Crawler.selector == crawler_selector)).one()
            product_query = product_query.where(Product.crawler_id == crawler.id)

        products = session.scalars(product_query).all()
        if not products:
            return

        benchmark_query = select(Benchmark)
        if benchmark_id is not None:
            benchmark_query = benchmark_query.where(Benchmark.id == benchmark_id)
            session.execute(delete(ProductBenchmark).where(ProductBenchmark.benchmark_id == benchmark_id))
        else:
            prod_ids = [p.id for p in products]
            session.execute(delete(ProductBenchmark).where(ProductBenchmark.product_id.in_(prod_ids)))

        benchmarks = session.scalars(benchmark_query).all()
        if not benchmarks:
            return

        prod_emb = []
        for p in products:
            if p.embedding is None:
                p.embedding = prompt_to_embedding(
                    _prompt(p.name, p.sku, p.category, p.units, p.price, p.amount, p.description)
                )
                session.commit()
            prod_emb.append(p.embedding)

        bench_emb = []
        for b in benchmarks:
            if b.embedding is None:
                b.embedding = prompt_to_embedding(
                    _prompt(b.name, b.sku, b.category, b.units, b.price, b.amount, b.description)
                )
                session.commit()
            bench_emb.append(b.embedding)

        prod_emb = np.array(prod_emb, dtype="float32")
        bench_emb = np.array(bench_emb, dtype="float32")

        index = faiss.IndexFlatIP(prod_emb.shape[1])
        index.add(prod_emb)  # type: ignore

        k = min(10, len(products))
        distances, indices = index.search(bench_emb, k)  # type: ignore

        threshold = 0.84
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
