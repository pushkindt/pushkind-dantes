import logging
from urllib.parse import urljoin

from bs4 import BeautifulSoup
from pushkind_crawlers.crawler.http import HTTPGetAIOHTTP
from pushkind_crawlers.crawler.protocols import Category, HTTPGet, Product

log = logging.getLogger(__name__)


class WebstoreParser101TeaRu:
    base_url: str = "https://101tea.ru/"

    def __init__(self, http_get: HTTPGet):
        self.http_get = http_get

    async def get_products(self, url: str) -> list[Product]:
        status, response = await self.http_get.get(url)
        if status != 200:
            return []

        soup = BeautifulSoup(response, "html.parser")
        product_cards = soup.find_all("div", {"class": "product-card"})
        products = []

        for card in product_cards:
            # Extract product name
            name_tag = card.find("p", class_="product-card__name")  # type: ignore
            name = name_tag.get_text(strip=True) if name_tag else "Unknown"

            # Extract product URL to derive SKU
            link_tag = name_tag.parent  # type: ignore
            href = link_tag.get("href") if link_tag else ""
            sku = href.strip("/").split("/")[-1] if href else "unknown"  # type: ignore

            # Extract price
            price_tag = card.find("span", class_="product-card__current-price cur")  # type: ignore
            price_tag = price_tag.find("span", class_="js-price-val") if price_tag else None  # type: ignore
            price_text = price_tag.get_text(strip=True) if price_tag else "0"
            # Remove non-numeric characters and convert to float
            price = float("".join(filter(str.isdigit, price_text))) if price_text else 0.0

            products.append(Product(sku=sku, name=name, price=price, url=urljoin(self.base_url, href) if href else ""))  # type: ignore

        return products

    async def get_categories(self) -> list[Category]:
        status, response = await self.http_get.get(self.base_url)
        if status != 200:
            return []
        soup = BeautifulSoup(response, "html.parser")
        category_links = soup.find_all("a", {"class": "catalog-nav__link"})
        result = [
            Category(
                id=link.get("href").split("/")[-2],  # type: ignore
                name=link.text.strip(),
                url=urljoin(self.base_url, link.get("href")),  # type: ignore
            )
            for link in category_links
            if link.get("href")  # type: ignore
        ]
        return result

    async def get_pages(self, url: str) -> list[str]:
        status, response = await self.http_get.get(url)
        result = [url]
        if status != 200:
            return result
        soup = BeautifulSoup(response, "html.parser")
        pagination = soup.find("div", {"class": "pagination"})
        if not pagination:
            return result
        pages = pagination.find_all("a", {"class": "pagination-links"})  # type: ignore
        if not pages:
            return result
        last_page_number = int(pages[-1].text.strip())
        page_url_template = pages[-1].get("href").rsplit("=")[0]  # type: ignore
        for page_number in range(1, last_page_number + 1):
            result.append(urljoin(self.base_url, f"{page_url_template}={page_number}"))
        return result


async def parse_101tea() -> list[tuple[Category, list[Product]]]:
    all_products = []
    parser_101 = WebstoreParser101TeaRu(http_get=HTTPGetAIOHTTP())
    categories = await parser_101.get_categories()
    for category in categories:
        log.info("Processing category: %s", category.name)
        categery_products = []
        try:
            pages = await parser_101.get_pages(category.url)
        except Exception:
            continue
        for page in pages:
            log.info("Processing page: %s", page)
            try:
                page_products = await parser_101.get_products(page)
            except Exception:
                continue
            categery_products += page_products
        all_products.append((category, categery_products))
    return all_products
