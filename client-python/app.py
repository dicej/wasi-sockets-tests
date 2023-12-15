# Workaround for https://github.com/bytecodealliance/componentize-py/issues/23:
from encodings import idna

import sys
import asyncio
import socket
import ipaddress
from ipaddress import IPv4Address, IPv6Address
from command import exports
from typing import Tuple, Sequence

class Run(exports.Run):
    def run(self):
        args = sys.argv[1:]
        if len(args) != 1:
            print(f"usage: tcp <address>:<port>", file=sys.stderr)
            exit(-1)

        addresses, port = resolve(args[0])
        asyncio.run(send_and_receive(addresses, port))

IPAddress = IPv4Address | IPv6Address
        
def resolve(address_and_port: str) -> Tuple[Sequence[IPAddress], int]:
    host, separator, port = address_and_port.rpartition(':')
    assert separator
    try:
        return ([ipaddress.ip_address(host.strip("[]"))], int(port))
    except ValueError:
        return (list(map(lambda tuple: ipaddress.ip_address(tuple[4][0]), socket.getaddrinfo(host, None))), int(port))
        
async def send_and_receive(addresses: Sequence[IPAddress], port: int):
    for address in addresses:
        try:
            rx, tx = await asyncio.open_connection(str(address), port)
        except:
            continue

        message = b"So rested he by the Tumtum tree" 
        tx.write(message)
        await tx.drain()

        data = await rx.read(1024)
        assert message == data

        tx.close()
        await tx.wait_closed()

        return

    raise Exception(f"unable to connect to {addresses}")
    
