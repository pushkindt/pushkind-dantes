from pushkind_crawlers.db import Crawler
from sqlalchemy import Column, create_engine, func, select
from sqlalchemy.orm import DeclarativeBase, Mapped, Session, mapped_column
from sqlalchemy.types import TIMESTAMP


def turn_off_processing(db_url: str, crawler_id: str):
    engine = create_engine(db_url)
    with Session(engine) as session:
        stmt = select(Crawler).where(Crawler.selector == crawler_id)
        crawler = session.scalars(stmt).one()
        crawler.processing = False
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
