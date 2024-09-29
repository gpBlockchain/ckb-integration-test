import subprocess
import re
import time

def parse_wrk_output(script_paths, test_urls):
    markdown_tables = []

    # Extracting the header
    header = "| Script | Test Duration | Target URL | Threads | Connections | Requests/Sec | Latency 50% | Latency 75% | Latency 90% | Latency 99% | Total Requests |"

    for script_path, test_url in zip(script_paths, test_urls):
        try:
            print(f"script_path:{script_path},test_url:{test_url}")
            # Run the shell command and capture the output
            output = subprocess.run(
                ["wrk", "-t1", "-c1", "-d5m", "-s", script_path, "--latency", test_url, "--timeout", "300s"],
                capture_output=True, text=True)

            # Extract relevant information from the output using regular expressions
            match = re.search(r"Running (.+) test @ (.+)", output.stdout)
            if match:
                test_duration = match.group(1)
                target_url = match.group(2)

            match = re.search(r"(\d+) threads and (\d+) connections", output.stdout)
            if match:
                threads = match.group(1)
                connections = match.group(2)

            match = re.search(
                r"50%\s+(\d+\.\d+.+)\s+75%\s+(\d+\.\d+.+)\s+90%\s+(\d+\.\d+.+)\s+99%\s+(\d+\.\d+.+)\s+(\d+) requests in (\d+\.\d+.+), (\d+\.\d+.+) read",
                output.stdout)
            if match:
                latency_50 = match.group(1)
                latency_75 = match.group(2)
                latency_90 = match.group(3)
                latency_99 = match.group(4)
                total_requests = match.group(5)
                test_duration_minutes = match.group(6)
                data_read = match.group(7)

            match = re.search(r"Requests/sec:\s+(\d+\.\d+)", output.stdout)
            if match:
                req_sec_avg = match.group(1)

            # Format the extracted information into a markdown table
            markdown_table = f"""| {script_path} | {test_duration} | {target_url} | {threads} | {connections} | {req_sec_avg} | {latency_50} | {latency_75} | {latency_90} | {latency_99} | {total_requests} |\n"""

            markdown_tables.append(markdown_table)
        except:
            print(f"stress failed :{script_path}")
        time.sleep(60)
    # Combine the header with the markdown tables
    combined_table = f"{header}\n{'|'.join(['-' * len(col) for col in header.split('|')])}\n{''.join(markdown_tables)}"

    # Write the combined markdown table to a file
    with open("wkr.md", "w") as file:
        file.write(combined_table.strip())

    print("Markdown table has been written to wkr.md")


script_paths = [
    "stress_test_scripts/get_cells_capacity_exact.1w.lua",
    "stress_test_scripts/get_cells_capacity_exact.2w.lua",
    "stress_test_scripts/get_cells_capacity_exact.4w.lua",
    "stress_test_scripts/get_cells_capacity_exact.8w.lua",
    "stress_test_scripts/get_cells_capacity_exact.16w.lua",
    "stress_test_scripts/get_cells_capacity_exact.32w.lua",
    "stress_test_scripts/get_cells_capacity_exact.64w.lua",
    "stress_test_scripts/get_cells_capacity_exact.128w.lua",
    "stress_test_scripts/get_cells_capacity_exact.256w.lua",
    "stress_test_scripts/get_cells_capacity_exact.512w.lua",
    "stress_test_scripts/get_cells_capacity_exact.1024w.lua",

    "stress_test_scripts/get_cells_exact.1w.0x64.lua",
    "stress_test_scripts/get_cells_exact.2w.0x64.lua",
    "stress_test_scripts/get_cells_exact.4w.0x64.lua",
    "stress_test_scripts/get_cells_exact.8w.0x64.lua",
    "stress_test_scripts/get_cells_exact.16w.0x64.lua",
    "stress_test_scripts/get_cells_exact.32w.0x64.lua",
    "stress_test_scripts/get_cells_exact.64w.0x64.lua",
    "stress_test_scripts/get_cells_exact.128w.0x64.lua",
    "stress_test_scripts/get_cells_exact.256w.0x64.lua",
    "stress_test_scripts/get_cells_exact.512w.0x64.lua",
    "stress_test_scripts/get_cells_exact.1024w.0x64.lua",

]
test_urls = [
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",

    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021",
    "http://172.31.28.209:8021"
]

parse_wrk_output(script_paths, test_urls)
