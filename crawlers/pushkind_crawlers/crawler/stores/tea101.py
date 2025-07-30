import logging
import re
from urllib.parse import urljoin

from bs4 import BeautifulSoup
from pushkind_crawlers.crawler.http import HTTPGetAIOHTTP
from pushkind_crawlers.crawler.protocols import Category, HTTPGet, Product

log = logging.getLogger(__name__)


class WebstoreParser101TeaRu:
    base_url: str = "https://101tea.ru//moskva/product/puer_shu_dai_7692_2020_g_blin_357_g/"

    def __init__(self, http_get: HTTPGet):
        self.http_get = http_get

    async def get_product(self, url: str) -> Product | None:
        status, response = await self.http_get.get(url)
        if status != 200:
            return None
        soup = BeautifulSoup(response, "html.parser")
        try:
            return Product(
                sku=soup.find("div", {"class": "product_art"}).find_all("span")[1].text.strip(),  # type: ignore
                name=soup.find("h1", {"itemprop": "name"}).text.strip(),  # type: ignore
                price=soup.find("span", {"class": "js-price-val"}).text.replace(" ", ""),  # type: ignore
                category=re.sub("\n+", "/", soup.find("div", {"class": "breadcrumbs"}).text.strip()),  # type: ignore
                units=soup.find("span", {"class": "product-card__calculus-unit"}).text.strip(),  # type: ignore
                amount=soup.find("span", {"class": "js-product-calc-value product-card__calculus-value"}).text.replace(" ", ""),  # type: ignore
                description=soup.find("div", {"data-js-catalog-tab-id": "about_product"}).text.strip(),  # type: ignore
                url=url,
            )
        except Exception as e:
            log.error(f"Error parsing product {url}: {e}")
            return None

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
            href = str(link_tag.get("href"))  # type: ignore

            if not href:
                log.warning(f"Product {name} has no href")
                continue

            product = await self.get_product(urljoin(self.base_url, href))

            if not product:
                log.warning(f"Product {name} at {href} cannot be parsed")
                continue

            products.append(product)  # type: ignore

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


async def parse_101tea() -> list[Product]:
    all_products = []
    parser_101 = WebstoreParser101TeaRu(http_get=HTTPGetAIOHTTP())
    categories = await parser_101.get_categories()
    for category in categories:
        log.info("Processing category: %s", category.name)
        category_products = []
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
            category_products += page_products
        all_products += category_products

    # remove duplicate products based on product.url
    unique_products = {p.url: p for p in all_products}.values()
    return list(unique_products)
