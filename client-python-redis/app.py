# Workaround for https://github.com/bytecodealliance/componentize-py/issues/23:
from encodings import idna

import sys
import asyncio
import socket
import ipaddress
import redis.asyncio as redis
from ipaddress import IPv4Address, IPv6Address
from command import exports
from typing import Tuple, Sequence

class Run(exports.Run):
    def run(self):
        args = sys.argv[1:]
        if len(args) != 1:
            print(f"usage: {sys.argv[0]} <address>:<port>", file=sys.stderr)
            exit(-1)

        asyncio.run(send_and_receive(args[0]))

async def resolve(address_and_port: str) -> Tuple[Sequence[IPv4Address | IPv6Address], int]:
    host, separator, port = address_and_port.rpartition(':')
    assert separator
    try:
        return ([ipaddress.ip_address(host.strip("[]"))], int(port))
    except ValueError:
        # Ideally, we'd use `await asyncio.get_event_loop().getaddrinfo(host,
        # None)` here, but that requires
        # `concurrent.futures.ThreadPoolExecutor`, which requires
        # multithreading.  In the future, we could patch `asyncio` to use
        # `wasi:sockets/ip-name-lookup` directly instead of going through
        # `getaddrinfo`, which would allow it to be async without
        # multithreading.
        addresses = socket.getaddrinfo(host, None)
        return (list(map(lambda tuple: ipaddress.ip_address(tuple[4][0]), addresses)), int(port))
        
async def send_and_receive(address: str):
    addresses, port = await resolve(address)

    for address in addresses:
        try:
            client = redis.Redis(host=str(address), port=port)

            await client.set("foo", b"bar")
            assert await client.get("foo") == b"bar"
        
            await client.aclose()
            return
        except:
            pass

    raise Exception(f"unable to connect to {addresses}")
    
