use std::fs::File;
use std::io::Write;
use crate::bench::RunReport;
use crate::watcher::{PoolReport};
use crate::stat::{BlockReport, Report};
use serde::{Deserialize, Serialize};
use crate::prometheus::MemoryUsageReport;


#[derive(Debug, Serialize, Deserialize)]
pub struct TotalReport {
    pub block_report: BlockReport,
    pub stat_report: Report,
    pub pool_report: PoolReport,
    pub run_report: RunReport,
    pub memory_usage_report: MemoryUsageReport,
}

pub fn generate_html_report(data: &TotalReport) -> String {
    let json_data = serde_json::to_string(data).unwrap();
    // build html report
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>ckb bench Example</title>
    <style>
        /* General styling */
        body {
            margin: 0;
            font-family: Arial, sans-serif;
            background-color: #f9f9f9;
        }

        /* Layout */
        .container {
            display: flex;
        }

        #sidebar {
            width: 20%;
            padding: 20px;
            background-color: #303030;
            color: #fff;
        }

        #content {
            width: 80%;
            padding: 20px;
            background-color: #fff;
            box-shadow: 0 0 5px rgba(0, 0, 0, 0.1);
        }

        /* Module styling */
        .module {
            margin-bottom: 20px;
        }

        .module details {
            cursor: pointer;
        }

        .module h3 {
            margin: 0 0 10px;
        }

        /* Module links styling */
        .module-link {
            display: block;
            color: #fff;
            text-decoration: none;
            padding: 5px 0;
            transition: background-color 0.3s;
        }

        .module-link.active {
            background-color: #444;
        }

        /* Module details styling */
        .module details summary::-webkit-details-marker {
            display: none;
        }

        table {
            border-collapse: collapse;
            width: 100%;
        }

        th, td {
            border: 1px solid black;
            padding: 8px;
            text-align: left;
        }

        th {
            background-color: #f2f2f2;
        }
    </style>
    <script src='https://cdn.jsdelivr.net/npm/chart.js'></script>
</head>
<body>
<div class='container'>
    <div id='sidebar'>
        <h2>CKB BENCH </h2>
        <ul>
            <li><a href='#module1' class='module-link' data-target='module1'>total report</a></li>
            <li><a href='#module2' class='module-link' data-target='module2'>Run Report</a></li>
            <li><a href='#module3' class='module-link' data-target='module3'>Pool Report</a></li>
            <li><a href='#module4' class='module-link' data-target='module4'>Block Report</a></li>
            <li><a href='#module5' class='module-link' data-target='module5'>Memory Usage Report</a></li>
        </ul>
    </div>

    <div id='content'>
        <h3>option</h3>
        <div>
            <label for='inputData'>add data:</label>
            <input type='text' id='inputData' placeholder='add ckb bench json data' style='width: 300px; height: 50px;'>
            <label for='groupSize'>group size for chart:</label>
            <input type='text' id='groupSize' placeholder='5'>
            <button id='showButton'>show</button>
        </div>
        <div style='height: 5px;'></div>
        <div>
            <label for='removeIdx'>remove data:</label>
            <input type='text' id='removeIdx' placeholder='0'>
            <button id='removeButton'>remove</button>
        </div>
        <div class='module' id='module1'>
            <h3>Total Report</h3>
            <details>
                <summary>detail</summary>
                <div id='stat_report'></div>
                <p></p>
            </details>
        </div>

        <div class='module' id='module2'>
            <h3>Run Report</h3>
            <details>
                <summary>details</summary>
                <div>
                    <canvas id='detail_delays_chart'></canvas>
                </div>
                <div>
                    <canvas id='detail_tps_chart'></canvas>
                </div>
            </details>
        </div>
        <div class='module' id='module3'>
            <h3>Pool Report</h3>
            <details>
                <summary>details</summary>
                <div>
                    <canvas id='pending_chart'></canvas>
                </div>
                <div>
                    <canvas id='orphan_chart'></canvas>
                </div>
                <div>
                    <canvas id='proposed_chart'></canvas>
                </div>
                <div>
                    <canvas id='block_number_chart'></canvas>
                </div>
            </details>
        </div>
        <div class='module' id='module4'>
            <h3>Block Report</h3>
            <details>
                <summary>details</summary>
                <div>
                    <canvas id='block_delays_chart'></canvas>
                </div>
                <div>
                    <canvas id='tps_chart'></canvas>
                </div>
                <div>
                    <canvas id='tx_size_chart'></canvas>
                </div>
                <div>
                    <canvas id='tx_number_chart'></canvas>
                </div>
            </details>
        </div>
        <div class='module' id='module5'>
            <h3>Memory Usage Report</h3>
            <details>
                <summary>details</summary>
                <div>
                    <canvas id='ckb_sys_mem_process_rss_chart'></canvas>
                </div>
                <div>
                    <canvas id='ckb_sys_mem_process_vms_chart'></canvas>
                </div>
            </details>
        </div>

    </div>
</div>

<script>
    const moduleLinks = document.querySelectorAll('.module-link');
    const modules = document.querySelectorAll('.module');
    let group_size = 5;
    moduleLinks.forEach(link => {
        link.addEventListener('click', e => {
            e.preventDefault();
            const targetModuleId = link.getAttribute('data-target');
            document.getElementById(targetModuleId).style.display = 'block';
            modules.forEach(module => {
                module.querySelector('details').removeAttribute('open');
            });
            document.getElementById(targetModuleId).querySelector('details').setAttribute('open', 'true');
            moduleLinks.forEach(moduleLink => {
                moduleLink.classList.remove('active');
            });
            link.classList.add('active');
            document.getElementById(targetModuleId).scrollIntoView();
        });
    });

    let jsonReports = [__replace__json__reports__];

    document.addEventListener('DOMContentLoaded', function () {
        const inputData = document.getElementById('inputData');
        const groupSizeData = document.getElementById('groupSize');
        const showButton = document.getElementById('showButton');
        const removeIdxButton = document.getElementById('removeButton');
        showButton.addEventListener('click', function () {
            jsonReports.push(JSON.parse(inputData.value))
            group_size = parseInt(groupSizeData.value)
            generateReport(group_size)

        });
        removeIdxButton.addEventListener('click', function () {
            const removeIndex = document.getElementById('removeIdx');
            jsonReports.splice(parseInt(removeIndex), 1);

            generateReport(group_size)
        })
    });

    generateReport(group_size)

    function generateReport(group_size) {
        // Total Report
        addStatReportTable(jsonReports, 'stat_report')

        // Run Report
        getChart(jsonReports, 'detail_delays_chart', 'run_report', 'tps', group_size)
        getChart(jsonReports, 'detail_tps_chart', 'run_report', 'delay_ms', group_size)

        // Pool report
        getChart(jsonReports, 'pending_chart', 'pool_report', 'pending', group_size)
        getChart(jsonReports, 'orphan_chart', 'pool_report', 'orphan', group_size)
        getChart(jsonReports, 'proposed_chart', 'pool_report', 'proposed', group_size)
        getChart(jsonReports, 'block_number_chart', 'pool_report', 'block_number', group_size)

        // Block Report
        getChart(jsonReports, 'block_delays_chart', 'block_report', 'block_delay_ms', group_size)
        getChart(jsonReports, 'tps_chart', 'block_report', 'tps', group_size)
        getChart(jsonReports, 'tx_size_chart', 'block_report', 'block_transaction_size', group_size)
        getChart(jsonReports, 'tx_number_chart', 'block_report', 'block_number', group_size)

        // memory usage report
        getChart(jsonReports, 'ckb_sys_mem_process_rss_chart', 'memory_usage_report', 'ckb_sys_mem_process_rss_mb', group_size)
        getChart(jsonReports, 'ckb_sys_mem_process_vms_chart', 'memory_usage_report', 'ckb_sys_mem_process_vms_mb', group_size)

    }

    function addStatReportTable(statReports, elementId) {
        if (statReports.length === 0) {
            return;
        }
        var reportTable = '<table><tr>';
        for (var key in statReports[0]['stat_report']) {
            if (statReports[0]['stat_report'].hasOwnProperty(key)) {
                reportTable = reportTable + '<tr>'
                reportTable = reportTable + '<td>' + key + '</td>'
                for (let i = 0; i < statReports.length; i++) {
                    reportTable = reportTable + '<td>' + statReports[i]['stat_report'][key] + '</td>';
                }
                reportTable = reportTable + '</tr>'
            }
        }
        reportTable = reportTable + '</tr></table>';
        document.getElementById(elementId).innerHTML = reportTable;

    }

    function getChart(jsonReports, elementById, dataBase, dataKey, group_size) {
        const comparisonChartDiv = document.getElementById(elementById);
        const sendTpsChartData = prepareChartData(jsonReports, dataBase, dataKey, group_size);
        const ctx = comparisonChartDiv.getContext('2d');
        const existingChart = Chart.getChart(comparisonChartDiv);
        if (existingChart) {
            existingChart.destroy();
        }
        new Chart(ctx, {
            type: 'line',
            data: sendTpsChartData,
            options: {
                responsive: true,
                maintainAspectRatio: false,
                scales: {
                    x: {
                        type: 'category',
                        beginAtZero: true
                    },
                    y: {
                        beginAtZero: true
                    }
                }, plugins: {
                    tooltip: {
                        callbacks: {
                            label: (context) => context.dataset.pointLabels[context.dataIndex]
                        }
                    }
                }
            },

        });
    }

    function prepareChartData(runReports, dataBase, dataKey, group_size) {
        console.log(`prepareChartData:${group_size}`)
        const chartData = {
            labels: [], // Will be populated with timestamp labels
            datasets: [] // Will be populated with report datasets
        };
        // const maxDataLength = Math.max(...jsonReports.map(report => report.run_report.send_tps.length))/5;
        // const alignedSendTpsData = new Array(maxDataLength).fill(null);
        runReports.forEach((report, index) => {
            let data = report[dataBase][dataKey];
            const maxData = Math.max(...data);
            const minData = Math.min(...data);
            const avgData = data.reduce((acc, val) => acc + val, 0) / data.length;
            data = averageGroupedData(data, group_size)

            let timestampData = averageGroupedData(report[dataBase].timestamp, group_size).map(ts => {
                return new Date(ts).toLocaleTimeString()
            });
            // Add timestamp labels
            if (index === 0) {
                chartData.labels = timestampData;
            }

            // Create dataset for each report
            chartData.datasets.push({
                pointLabels: data.map(value =>
                    `Max: ${maxData.toFixed(2)}\nMin: ${minData.toFixed(2)}\nAvg: ${avgData.toFixed(2)}\nCurrent: ${value.toFixed(2)}`
                ),
                label: `${dataKey}-${index}`,
                data: data,
                borderColor: generateDistinctColors(index),
                fill: true
            });
        });

        return chartData;
    }

    // Function to generate random color
    function getRandomColor() {
        return `rgb(${Math.random() * 255}, ${Math.random() * 255}, ${Math.random() * 255})`;
    }

    function generateDistinctColors(numColors) {
        const predefinedColors = [
            'rgb(255, 99, 132)',
            'rgb(54, 162, 235)',
            'rgb(255, 205, 86)',
            'rgb(75, 192, 192)',
            'rgb(153, 102, 255)',
            'rgb(123, 2, 15)',
            'rgb(13, 2, 15)',
            'rgb(23, 25, 115)',
            'rgb(165,140,140)',
        ];
        if (numColors > predefinedColors.length - 1) {
            return getRandomColor()
        }
        return predefinedColors[numColors];
    }

    function averageGroupedData(data, groupSize) {
        const groupedData = [];
        const numGroups = Math.ceil(data.length / groupSize);

        for (let i = 0; i < numGroups; i++) {
            const startIndex = i * groupSize;
            const endIndex = Math.min(startIndex + groupSize, data.length);
            const group = data.slice(startIndex, endIndex);

            // Using BigInt for accumulation to handle large data points
            const sum = group.reduce((sum, value) => sum + (value), (0));
            const average = sum / (group.length);
            groupedData.push(Number(average)); // Convert back to Number after calculations
        }

        return groupedData;
    }
</script>
</body>
</html>

"#;
    html.replace("__replace__json__reports__", json_data.as_str())
}

pub fn write_to_file(filename: &str, content: &str) -> std::io::Result<()> {
    let mut file = File::create(filename)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}