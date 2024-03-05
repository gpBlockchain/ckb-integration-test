from discord import Embed
import re
import discord
import sys
import os
import asyncio

# export RESULT=`head -n 5 demo.md | tail -n +3`
variable_value = os.environ.get("RESULT")

intents = discord.Intents.default()
intents.message_content = True

CHANNEL_ID = 1097347489490800755
TOKEN = os.environ.get("TOKEN")

# 声明一个客户端
client = discord.Client(intents=intents)

# 将Markdown表格内容转换为嵌入式表格
def markdown_table_to_embed(content):
    lines = content.strip().split('\n')
    headers = lines[0].strip().split('|')[1:-1]
    rows = [line.strip().split('|')[1:-1] for line in lines[2:]]

    embed = Embed(title="性能测试结果")
    for row in rows:
        for i, field in enumerate(row):
            embed.add_field(name=headers[i].strip(), value=field.strip(), inline=True)

    return embed

# 当客户端准备好时触发的事件处理器
@client.event
async def on_ready():
    print(f'已登录为 {client.user}')

    markdown_table = variable_value
    print(markdown_table)
    # 将Markdown表格转换为嵌入式表格并发送到指定的频道
    channel = client.get_channel(CHANNEL_ID)  # 替换为你要发送消息的频道 ID
    embed = markdown_table_to_embed(markdown_table)
    await channel.send(embed=embed)

    # 等待一段时间后再关闭客户端
    await asyncio.sleep(5)  # 5 秒钟
    await client.close()

# 运行客户端
client.run(TOKEN) 
