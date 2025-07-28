import aiohttp
import requests


class HTTPGetRequests:
    async def get(self, url: str) -> tuple[int, str]:
        response = requests.get(url, timeout=10)
        response.raise_for_status()
        return response.status_code, response.text


class HTTPGetAIOHTTP:
    async def get(self, url: str) -> tuple[int, str]:
        async with aiohttp.ClientSession() as session:
            async with session.get(url) as response:
                return response.status, await response.text()
