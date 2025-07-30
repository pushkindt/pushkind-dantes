import aiohttp
import asyncio
import requests


class HTTPGetRequests:
    async def get(self, url: str) -> tuple[int, str]:
        response = await asyncio.to_thread(requests.get, url, timeout=10)
        response.raise_for_status()
        return response.status_code, response.text


class HTTPGetAIOHTTP:
    def __init__(self) -> None:
        self._session: aiohttp.ClientSession | None = None

    async def __aenter__(self) -> "HTTPGetAIOHTTP":
        self._session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc, tb) -> None:
        if self._session is not None:
            await self._session.close()
            self._session = None

    async def get(self, url: str) -> tuple[int, str]:
        if self._session is None:
            self._session = aiohttp.ClientSession()
        async with self._session.get(url) as response:
            return response.status, await response.text()
