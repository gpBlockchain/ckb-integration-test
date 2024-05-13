import os
import tarfile
import json
import re

from qiniu import Auth, put_file, etag

# Qiniu configuration
# QINIU_ACCESS_KEY = ""
# QINIU_SECRET_KEY = ""
QINIU_ACCESS_KEY = os.environ.get("ACCESS_KEY")
QINIU_SECRET_KEY = os.environ.get("SECRET_KEY")
QINIU_BUCKET_NAME = "acceptance-test"

# Paths
TAR_FILE_PATH = "job/benchmark-in-10h/ansible/logs/demo.tar.gz"
TEMP_DIRECTORY = "job/benchmark-in-10h/temp"
GRAFANA_BASE_URL = "https://grafana-monitor.nervos.tech/d/pThsj6xVz/test?orgId=1&var-url=18.163.87.248:8100&var-url=18.163.155.251:8100&var-url=18.166.86.54:8100"
GITHUB_LOGS_BASE_URL = "http://github-test-logs.ckbapp.dev/ckb/ckb-bench/reports"
MD_PATH = "demo.md"

def upload_file_to_qiniu(access_key, secret_key, bucket_name, key, local_file):
    q = Auth(access_key, secret_key)
    token = q.upload_token(bucket_name, key, 3600)
    ret, info = put_file(token, key, local_file, version='v2')
    print(info)
    assert ret['key'] == key
    assert ret['hash'] == etag(local_file)


def extract_file(filename, path):
    """
        Extract a compressed file to the specified path.

        Args:
            filename (str): The name of the compressed file.
            path (str): The path to extract the files to.
    """
    temp_path = os.path.join(path, 'temp')
    os.makedirs(temp_path, exist_ok=True)

    with tarfile.open(filename, 'r:gz') as tar_ref:
        tar_ref.extractall(temp_path)

    # Move the extracted files from the subdirectory to the target directory
    extracted_subdir = os.path.join(temp_path, os.listdir(temp_path)[0])
    for file in os.listdir(extracted_subdir):
        os.rename(os.path.join(extracted_subdir, file), os.path.join(path, file))

    # Remove the downloaded zip or tar.gz file and the temporary directory
    # os.remove(filename)
    os.rmdir(extracted_subdir)
    os.rmdir(temp_path)


# 获取所有的 json 文件
def get_all_json_files(directory_path):
    json_files = []

    for root, dirs, files in os.walk(directory_path):
        for file in files:
            if file.endswith(".json") and file.startswith("report"):
                # 如果文件扩展名是.json，将其路径添加到json_files列表中
                json_files.append(os.path.join(root, file))

    return json_files


# 获取测试的ckb版本号
def get_test_json_version(json_file_path):
    with open(json_file_path, "r") as json_file:
        # 使用json.load()方法将JSON文件的内容反序列化为字典
        json_data = json.load(json_file)
    match = re.search(r'\(([^ ]+)', json_data['stat_report']['ckb_version'])
    if match:
        result = match.group(1)
        print(result)
        return result
    else:
        print("未找到匹配的内容")
        return json_data['stat_report']['ckb_version']



def get_bench_timestamp_grafana(json_file_path_0_50w_file,json_file_path_50w_0_file):
    json_0_50w_json_data = json_file_path_0_50w_file.split("/")[-1].split(".")
    json_50w_0_json_data = json_file_path_50w_0_file.split("/")[-1].split(".")

    return f"https://grafana-monitor.nervos.tech/d/pThsj6xVz/test?orgId=1&var-url=18.163.87.248:8100&var-url=18.163.155.251:8100&var-url=18.166.86.54:8100&from={json_0_50w_json_data[-3]}000&to={json_50w_0_json_data[-2]}000"


# 生成makerdown 文本
def json_to_key_value_md_table(json_data):
    # 检查输入是否为一个包含字典的 JSON 数组
    if not isinstance(json_data, list) or not all(isinstance(item, dict) for item in json_data):
        return "Invalid input. Please provide a JSON array of dictionaries."
    md_table = "### ckb-pool-test \n\n"
    # 初始化 Markdown 表格头部
    md_table += "|"

    # 遍历字典中的每个键值对
    for key, value in json_data[0].items():
        # 将键和值添加到 Markdown 表格中
        md_table += f" {key} |"
    md_table +="\n |"
        # 遍历字典中的每个键值对
    for _, _ in json_data[0].items():
        # 将键和值添加到 Markdown 表格中
        md_table += f" --- |"
    md_table +="\n |"
    # 遍历 JSON 数组中的每个字典
    for item in json_data:
        # 遍历字典中的每个键值对，并将它们作为行添加到 Markdown 表格中
        for key, value in item.items():
            if str(value).startswith("http"):
                md_table +=f"[link]({value}) |"
            else:
                md_table += f"{value} |"
        md_table +="\n |"
    md_table = md_table[:-1]
    md_table += " <hr/>\n"
    md_table +="\n[Explanation of Terms](https://github.com/gpBlockchain/ckb-integration-test/tree/gp/cell-query/ckb-cell-query#interpretation-of-test-results)"
    return md_table

if __name__ == '__main__':

    extract_file(TAR_FILE_PATH, TEMP_DIRECTORY)
    # 获取所有的json 文件
    json_files = get_all_json_files(TEMP_DIRECTORY)
    json_data = []
    bench_0_50w_json_file = ""
    bench_50w_0_json_file = ""
    for json_file in json_files:
        if "report.2000" in json_file:
            bench_0_50w_json_file = json_file
        if "report.1" in json_file:
            bench_50w_0_json_file = json_file

    ckb_version = get_test_json_version(bench_50w_0_json_file)
    for json_file in json_files:
        # 上传html文件
        upload_file_to_qiniu(
            QINIU_ACCESS_KEY,
            QINIU_SECRET_KEY,
            QINIU_BUCKET_NAME,
            f'ckb/ckb-bench/reports/{ckb_version}/{os.path.basename(json_file).replace("json", "html")}',
            json_file.replace("json", "html")
        )

        # 上传json文件
        upload_file_to_qiniu(
            QINIU_ACCESS_KEY,
            QINIU_SECRET_KEY,
            QINIU_BUCKET_NAME,
            f'ckb/ckb-bench/reports/{ckb_version}/{os.path.basename(json_file)}',
            json_file
        )
    grafana_url = get_bench_timestamp_grafana(bench_0_50w_json_file,bench_50w_0_json_file)

    # 获取html链接
    report_50w_0_url = f'{GITHUB_LOGS_BASE_URL}/{ckb_version}/{os.path.basename(bench_50w_0_json_file).replace("json", "html")}'
    report_0_50w_url = f'{GITHUB_LOGS_BASE_URL}/{ckb_version}/{os.path.basename(bench_0_50w_json_file).replace("json", "html")}'

    # 0_50w_client_send_tps
    with open(bench_0_50w_json_file, "r") as json_file:
        # 使用json.load()方法将JSON文件的内容反序列化为字典
        bench_0_50w_json_data = json.load(json_file)
    bench_0_50w_client_send_tps = bench_0_50w_json_data['stat_report']['client_send_tps']

    # 50w_0_transactions_per_second average_block_time_ms max_pending_size min_pending_size
    with open(bench_50w_0_json_file, "r") as json_file:
        # 使用json.load()方法将JSON文件的内容反序列化为字典
        bench_50w_0_json_data = json.load(json_file)
    bench_50w_0_transactions_per_second = bench_50w_0_json_data['stat_report']['transactions_per_second']
    bench_50w_0_average_block_time_ms = bench_50w_0_json_data['stat_report']['average_block_time_ms']
    bench_50w_0_max_pending_size = bench_50w_0_json_data['pool_report']['pending'][0]
    bench_50w_0_min_pending_size = bench_50w_0_json_data['pool_report']['pending'][-1]


    # 生成 md 语法
    result = {
        'ckb_version':ckb_version,
        '0_50w_client_send_tps':bench_0_50w_client_send_tps,
        '50w_0_transactions_per_second':bench_50w_0_transactions_per_second,
        'average_block_time_ms':bench_50w_0_average_block_time_ms,
        'max_pending_size':bench_50w_0_max_pending_size,
        'min_pending_size':bench_50w_0_min_pending_size,
        'grafana':grafana_url,
        'tx_pool_0_50w_report':report_0_50w_url,
        'tx_pool_50w_0_report':report_50w_0_url,
    }
    md = json_to_key_value_md_table([result])
    with open(MD_PATH, "w") as f:
        f.write(md)