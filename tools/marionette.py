"""Small synchronous client for Firefox's documented development protocol."""
import json
import socket


class Marionette:
    def __init__(self, port):
        self.socket = socket.create_connection(("127.0.0.1", port), timeout=45)
        self.sequence = 0
        greeting = self._receive()
        if greeting.get("marionetteProtocol") != 3:
            self.close()
            raise RuntimeError("Unsupported Marionette protocol")

    def _exact(self, length):
        chunks = bytearray()
        while len(chunks) < length:
            data = self.socket.recv(length - len(chunks))
            if not data:
                raise RuntimeError("Firefox closed its automation connection")
            chunks.extend(data)
        return chunks

    def _receive(self):
        digits = bytearray()
        while True:
            byte = self._exact(1)
            if byte == b":":
                break
            if not byte.isdigit() or len(digits) >= 9:
                raise RuntimeError("Invalid Marionette frame")
            digits.extend(byte)
        length = int(digits)
        if length > 16 * 1024 * 1024:
            raise RuntimeError("Oversized Marionette response")
        return json.loads(self._exact(length))

    def command(self, name, params=None):
        self.sequence += 1
        data = json.dumps([0, self.sequence, name, params or {}]).encode()
        self.socket.sendall(str(len(data)).encode() + b":" + data)
        response = self._receive()
        if response[0:2] != [1, self.sequence]:
            raise RuntimeError("Unexpected Marionette response")
        if response[2]:
            raise RuntimeError(f"{name}: {response[2]['message']}")
        return response[3]

    def close(self):
        self.socket.close()
