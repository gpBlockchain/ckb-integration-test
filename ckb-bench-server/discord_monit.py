import asyncio
import json
import os
import re
from pathlib import Path

try:
    import discord
    from discord import Embed
except ModuleNotFoundError:
    discord = None
    Embed = None

# export RESULT=`head -n 5 demo.md | tail -n +3`
variable_value = os.environ.get("RESULT", "")

CHANNEL_ID = 1214508575062360074
TOKEN = os.environ.get("TOKEN")
BASE_DIR = Path(__file__).resolve().parent
REPORT_DIR = Path(os.environ.get("BENCH_REPORT_DIR", BASE_DIR / "job/benchmark-in-10h/temp"))
APP_DATA_PATH = Path(os.environ.get("APP_DATA_PATH", BASE_DIR.parent / "app/public/data.json"))
DATA_FIELDS = [
    "id",
    "date",
    "n_nodes",
    "n_inout",
    "ckb_version",
    "ckb_version_short",
    "delay_time_ms",
    "from_block_number",
    "to_block_number",
    "transactions_per_second",
    "transactions_size_per_second",
    "average_block_transactions",
    "average_block_transactions_size",
    "average_block_time_ms",
    "total_transactions",
    "total_transactions_size",
    "set_send_tps",
    "client_send_tps",
    "grafana_link",
    "report_link",
]

if discord is not None:
    intents = discord.Intents.default()
    intents.message_content = True
    client = discord.Client(intents=intents)
else:
    client = None

# 将Markdown表格内容转换为嵌入式表格
def markdown_table_to_embed(content):
    if Embed is None:
        raise RuntimeError("discord package is not installed")

    lines = content.strip().split('\n')
    headers = [h.strip() for h in lines[0].strip().split('|')[1:-1]]
    rows = [line.strip().split('|')[1:-1] for line in lines[2:]]

    allowed_fields = {'ckb_version', 'transactions_per_second'}
    embed = Embed(title="性能测试结果")
    for row in rows:
        for i, field in enumerate(row):
            if headers[i] in allowed_fields:
                embed.add_field(name=headers[i], value=field.strip(), inline=True)

    return embed


def extract_date(ckb_version):
    match = re.search(r"\d{4}-\d{2}-\d{2}", ckb_version or "")
    return match.group(0) if match else ""


def extract_version_short(ckb_version):
    if not ckb_version:
        return ""
    return ckb_version.split("(")[0].strip()


def normalize_record(stat_report):
    record = {
        "id": "",
        "date": extract_date(stat_report.get("ckb_version", "")),
        "n_nodes": stat_report.get("n_nodes", 0),
        "n_inout": stat_report.get("n_inout", 0),
        "ckb_version": stat_report.get("ckb_version", ""),
        "ckb_version_short": extract_version_short(stat_report.get("ckb_version", "")),
        "delay_time_ms": stat_report.get("delay_time_ms", 0),
        "from_block_number": stat_report.get("from_block_number", 0),
        "to_block_number": stat_report.get("to_block_number", 0),
        "transactions_per_second": stat_report.get("transactions_per_second", 0),
        "transactions_size_per_second": stat_report.get("transactions_size_per_second", 0),
        "average_block_transactions": stat_report.get("average_block_transactions", 0),
        "average_block_transactions_size": stat_report.get("average_block_transactions_size", 0),
        "average_block_time_ms": stat_report.get("average_block_time_ms", 0),
        "total_transactions": stat_report.get("total_transactions", 0),
        "total_transactions_size": stat_report.get("total_transactions_size", 0),
        "set_send_tps": stat_report.get("set_send_tps", 0),
        "client_send_tps": stat_report.get("client_send_tps", 0),
        "grafana_link": stat_report.get("grafana", ""),
        "report_link": stat_report.get("report", ""),
    }
    return {field: record[field] for field in DATA_FIELDS}


def get_report_json_files(directory):
    if not directory.exists():
        return []
    return sorted(directory.rglob("report*.json"))


def load_new_records():
    records = []
    for json_file in get_report_json_files(REPORT_DIR):
        with json_file.open("r", encoding="utf-8") as file:
            json_data = json.load(file)
        stat_report = json_data.get("stat_report", {})
        if stat_report:
            records.append(normalize_record(stat_report))
    return sorted(records, key=lambda item: item["n_inout"])


def dedupe_key(record):
    return (
        record.get("date", ""),
        record.get("ckb_version", ""),
        record.get("n_nodes", 0),
        record.get("n_inout", 0),
        record.get("report_link", ""),
    )


def load_existing_records():
    if not APP_DATA_PATH.exists():
        return []
    with APP_DATA_PATH.open("r", encoding="utf-8") as file:
        records = json.load(file)
    normalized_records = []
    for record in records:
        normalized = {field: record.get(field, "") for field in DATA_FIELDS}
        normalized_records.append(normalized)
    return normalized_records


def write_records(records):
    APP_DATA_PATH.parent.mkdir(parents=True, exist_ok=True)
    with APP_DATA_PATH.open("w", encoding="utf-8") as file:
        json.dump(records, file, ensure_ascii=False, indent=2)
        file.write("\n")


def update_app_data():
    new_records = load_new_records()
    if not new_records:
        print(f"No report JSON files found under {REPORT_DIR}")
        return

    existing_records = load_existing_records()
    merged_records = []
    seen = set()

    for record in new_records + existing_records:
        key = dedupe_key(record)
        if key in seen:
            continue
        seen.add(key)
        merged_records.append(record)

    for index, record in enumerate(merged_records, start=1):
        record["id"] = str(index)

    write_records(merged_records)
    print(f"Updated {APP_DATA_PATH} with {len(new_records)} new record(s)")


async def maybe_send_discord_message():
    if discord is None:
        print("discord package is not installed, skip sending Discord notification")
        return

    if not TOKEN:
        print("TOKEN is not set, skip sending Discord notification")
        return

    markdown_table = variable_value.strip()
    if not markdown_table:
        print("RESULT is empty, skip sending Discord notification")
        return

    await client.start(TOKEN)

if client is not None:
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

async def main():
    update_app_data()
    await maybe_send_discord_message()


if __name__ == "__main__":
    asyncio.run(main())
