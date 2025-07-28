import asyncio
import logging

import zmq
import zmq.asyncio
from pushkind_dantes.crawler.protocols import Category, Product
from pushkind_dantes.crawler.stores.tea101 import parse_101tea

ctx = zmq.asyncio.Context()
log = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO)

running_crawlers: set[str] = set()


def products_to_csv(all_products: list[tuple[Category, list[Product]]], file_name: str):
    with open(file_name, "w", encoding="utf-8") as f:
        for category, products in all_products:
            for product in products:
                f.write(f"{category.name},{product.sku},{product.name},{product.price},{product.url}\n")


def log_task_exception(task: asyncio.Task):
    try:
        task.result()
    except Exception as e:
        log.exception("Exception in crawler task: %s", e)


async def consumer():

    async def handle_message(crawler_id: str):
        try:
            if crawler_id == "101tea":
                log.info("Handling: %s", crawler_id)
                products = await parse_101tea()
                products_to_csv(products, f"assets/{crawler_id}.csv")
                log.info("Done processing: %s → %d products", crawler_id, len(products))
            else:
                log.error("Unknown crawler: %s", crawler_id)
                raise ValueError(f"Unknown crawler: {crawler_id}")
        finally:
            running_crawlers.discard(crawler_id)

    socket = ctx.socket(zmq.PULL)
    socket.bind("tcp://0.0.0.0:5555")
    log.info("Waiting for messages...")
    while True:
        crawler_id = await socket.recv()
        crawler_id = crawler_id.decode("utf-8")
        if crawler_id in running_crawlers:
            log.warning("Crawler already running: %s. Skipping.", crawler_id)
            continue
        running_crawlers.add(crawler_id)
        task = asyncio.create_task(handle_message(crawler_id))
        task.add_done_callback(log_task_exception)


def main():
    asyncio.run(consumer())


if __name__ == "__main__":
    main()
