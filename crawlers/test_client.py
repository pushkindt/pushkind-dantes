import asyncio

import zmq
import zmq.asyncio

ctx = zmq.asyncio.Context()


async def producer():
    socket = ctx.socket(zmq.PUSH)
    socket.connect("tcp://localhost:5555")

    msg = "101tea"
    print(f"[Producer] Sending: {msg}")
    await socket.send_string(msg)
    await asyncio.sleep(0.1)
    socket.close()


asyncio.run(producer())
