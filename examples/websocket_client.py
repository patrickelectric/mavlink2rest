#!/usr/bin/env python3
import argparse
import asyncio
import aiohttp

async def start_client(url: str) -> None:
    ws = await aiohttp.ClientSession().ws_connect(url, autoclose=False, autoping=False)
    test = 0
    async def dispatch():
        while True:
            test += 1
            await ws.send_str('''{
  "header": {
    "system_id": 255,
    "component_id": 240,
    "sequence": {test}
  },
  "message": {
    "type":"COMMAND_LONG",
    "param1": 1.0,
    "param2": 0.0,"param3":0.0,"param4":0.0,"param5":0.0,"param6":0.0,"param7":0.0,
    "command": {
      "type": "MAV_CMD_COMPONENT_ARM_DISARM"
    },
    "target_system": 1,
    "target_component": 1,
    "confirmation": 1
  }
}''')
            msg = await ws.receive()

            if msg.type == aiohttp.WSMsgType.TEXT:
                print("Text: ", msg.data.strip())
            elif msg.type == aiohttp.WSMsgType.BINARY:
                print("Binary: ", msg.data)
            elif msg.type == aiohttp.WSMsgType.PING:
                await ws.pong()
            elif msg.type == aiohttp.WSMsgType.PONG:
                print("Pong received")
            else:
                if msg.type == aiohttp.WSMsgType.CLOSE:
                    await ws.close()
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    print("Error during receive %s" % ws.exception())
                elif msg.type == aiohttp.WSMsgType.CLOSED:
                    pass

                break

    await dispatch()


ARGS = argparse.ArgumentParser(
    description="websocket console client for wssrv.py example."
)
ARGS.add_argument(
    "--filter",
    action="store",
    dest="filter",
    default=".*",
    help="Regex filter or message name used on websocket: ATTITUDE,HEARTBEAT,RAW_IMU",
)
ARGS.add_argument(
    "--url",
    action="store",
    dest="url",
    default="http://0.0.0.0:8088/",
    help="Websocket address, follow the format: http://0.0.0.0:8088",
)

if __name__ == "__main__":
    args = ARGS.parse_args()

    loop = asyncio.new_event_loop()
    loop.run_until_complete(
        start_client(
            args.url + "v1/ws/mavlink" + f"?filter={args.filter}"
        )
    )
