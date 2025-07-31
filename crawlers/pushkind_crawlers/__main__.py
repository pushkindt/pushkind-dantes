import asyncio
import logging
import os

import zmq
import zmq.asyncio
from dotenv import load_dotenv
from pushkind_crawlers.crawler.stores.tea101 import parse_101tea
from pushkind_crawlers.db import save_products, update_benchmark_associations

ctx = zmq.asyncio.Context()
log = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO)

running_crawlers: set[str] = set()

crawlers_map = {
    "101tea": parse_101tea,
}


def log_task_exception(task: asyncio.Task):
    try:
        task.result()
    except Exception as e:
        log.exception("Exception in crawler task: %s", e)


async def consumer(zmq_address: str, db_url: str):

    async def handle_message(crawler_selector: str):
        try:
            if crawler_selector.isnumeric():
                log.info("Handling benchmark: %s", crawler_selector)
                update_benchmark_associations(db_url, benchmark_id=int(crawler_selector))
            elif crawler_selector in crawlers_map:
                log.info("Handling crawler: %s", crawler_selector)
                products = []
                products = await crawlers_map[crawler_selector]()
                save_products(db_url, crawler_selector, products)
                if products:
                    update_benchmark_associations(db_url, crawler_selector)
                log.info("Done processing: %s → %d products", crawler_selector, len(products))
            else:
                log.error("Unknown crawler: %s", crawler_selector)
                raise ValueError(f"Unknown crawler: {crawler_selector}")
        finally:
            running_crawlers.discard(crawler_selector)

    socket = ctx.socket(zmq.PULL)
    socket.bind(zmq_address)
    log.info("Waiting for messages...")
    while True:
        crawler_selector = await socket.recv()
        crawler_selector = crawler_selector.decode("utf-8")
        if crawler_selector in running_crawlers:
            log.warning("Crawler already running: %s. Skipping.", crawler_selector)
            continue
        running_crawlers.add(crawler_selector)
        task = asyncio.create_task(handle_message(crawler_selector))
        task.add_done_callback(log_task_exception)


def main():
    load_dotenv()
    db_url = os.getenv("DATABASE_URL")
    if not db_url:
        raise ValueError("DATABASE_URL not set")
    db_url = f"sqlite:///{db_url}"
    zmq_address = os.getenv("ZMQ_ADDRESS") or "tcp://0.0.0.0:5555"
    asyncio.run(consumer(zmq_address, db_url))


if __name__ == "__main__":
    main()
