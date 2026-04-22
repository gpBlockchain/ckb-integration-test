import { useState, useEffect, useMemo, useCallback } from 'react'
import type { BenchData, FilterConfig } from '../types/bench'
import {
  ChevronDown,
  ChevronUp,
  ExternalLink,
  Activity,
  GitBranch,
  Calendar,
} from 'lucide-react'

const FILTER_OPTIONS: { value: FilterConfig; label: string }[] = [
  { value: '3x1', label: '3 \u00d7 1' },
  { value: '3x2', label: '3 \u00d7 2' },
  { value: '3x5', label: '3 \u00d7 5' },
  { value: '3x10', label: '3 \u00d7 10' },
  { value: 'all', label: 'All' },
]

const ITEMS_PER_PAGE = 10

function getConfigLabel(n_nodes: number, n_inout: number): string {
  return `${n_nodes} \u00d7 ${n_inout}`
}

function formatNumber(num: number): string {
  return num.toLocaleString('en-US')
}

export default function Home() {
  const [data, setData] = useState<BenchData[]>([])
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState<FilterConfig>('3x1')
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [currentPage, setCurrentPage] = useState(1)

  useEffect(() => {
    fetch('./data.json')
      .then((res) => res.json())
      .then((json: BenchData[]) => {
        setData(json)
        setLoading(false)
      })
      .catch((err) => {
        console.error('Failed to load data:', err)
        setLoading(false)
      })
  }, [])

  const filteredData = useMemo(() => {
    if (filter === 'all') return data
    const [nodes, inout] = filter.split('x').map(Number)
    return data.filter((d) => d.n_nodes === nodes && d.n_inout === inout)
  }, [data, filter])

  const totalPages = Math.ceil(filteredData.length / ITEMS_PER_PAGE)

  const paginatedData = useMemo(() => {
    const start = (currentPage - 1) * ITEMS_PER_PAGE
    return filteredData.slice(start, start + ITEMS_PER_PAGE)
  }, [filteredData, currentPage])

  const latest3x1 = useMemo(() => {
    return data
      .filter((d) => d.n_nodes === 3 && d.n_inout === 1)
      .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())[0]
  }, [data])

  const handleToggleExpand = useCallback((id: string) => {
    setExpandedId((prev) => (prev === id ? null : id))
  }, [])

  const handleFilterChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      setFilter(e.target.value as FilterConfig)
      setCurrentPage(1)
      setExpandedId(null)
    },
    []
  )

  const handlePageChange = useCallback((page: number) => {
    setCurrentPage(page)
    setExpandedId(null)
  }, [])

  const showingFrom = filteredData.length === 0 ? 0 : (currentPage - 1) * ITEMS_PER_PAGE + 1
  const showingTo = Math.min(currentPage * ITEMS_PER_PAGE, filteredData.length)

  if (loading) {
    return (
      <div className="min-h-[100dvh] flex items-center justify-center" style={{ background: '#f8f9fa' }}>
        <div className="text-[#636e72] text-sm">Loading...</div>
      </div>
    )
  }

  return (
    <div className="min-h-[100dvh]" style={{ background: '#f8f9fa' }}>
      <div className="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* ===== Header ===== */}
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-8">
          <div>
            <h1
              className="text-[28px] font-bold tracking-tight"
              style={{ color: '#2d3436' }}
            >
              CKB Bench Daily Report
            </h1>
            <p className="text-sm mt-1" style={{ color: '#636e72' }}>
              Daily benchmark results for CKB node performance
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Calendar size={16} style={{ color: '#636e72' }} />
            <select
              value={filter}
              onChange={handleFilterChange}
              className="text-sm rounded-lg border px-3 py-2 outline-none cursor-pointer transition-shadow focus:ring-2"
              style={{
                borderColor: '#dfe6e9',
                background: '#ffffff',
                color: '#2d3436',
              }}
            >
              {FILTER_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* ===== Stats Cards ===== */}
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-8">
          {/* Latest TPS */}
          <div
            className="rounded-xl border p-5 transition-shadow hover:shadow-md"
            style={{
              background: '#ffffff',
              borderColor: '#dfe6e9',
            }}
          >
            <div className="flex items-center gap-2 mb-2">
              <Activity size={16} style={{ color: '#00b894' }} />
              <span className="text-xs font-semibold uppercase tracking-[0.5px]" style={{ color: '#636e72' }}>
                Latest TPS
              </span>
            </div>
            <div className="flex items-baseline gap-2">
              <span
                className="text-[28px] font-bold"
                style={{ color: '#00b894' }}
              >
                {latest3x1 ? formatNumber(latest3x1.transactions_per_second) : '--'}
              </span>
              <span className="text-sm" style={{ color: '#636e72' }}>
                tx/s
              </span>
            </div>
            <div className="text-xs mt-1" style={{ color: '#636e72' }}>
              {latest3x1 ? `3 \u00d7 1 config \u00b7 ${latest3x1.date}` : ''}
            </div>
          </div>

          {/* Latest Version */}
          <div
            className="rounded-xl border p-5 transition-shadow hover:shadow-md"
            style={{
              background: '#ffffff',
              borderColor: '#dfe6e9',
            }}
          >
            <div className="flex items-center gap-2 mb-2">
              <GitBranch size={16} style={{ color: '#0984e3' }} />
              <span className="text-xs font-semibold uppercase tracking-[0.5px]" style={{ color: '#636e72' }}>
                Latest Version
              </span>
            </div>
            <div className="flex items-center gap-2 mt-3">
              <span
                className="inline-flex items-center px-3 py-1 rounded-full text-[13px] font-medium"
                style={{
                  background: '#e8f5e9',
                  color: '#2e7d32',
                }}
              >
                {latest3x1 ? latest3x1.ckb_version_short : '--'}
              </span>
            </div>
            <div className="text-xs mt-2" style={{ color: '#636e72' }}>
              {latest3x1 ? `Tested on ${latest3x1.date}` : ''}
            </div>
          </div>
        </div>

        {/* ===== Main Table ===== */}
        <div
          className="rounded-xl border overflow-hidden"
          style={{
            background: '#ffffff',
            borderColor: '#dfe6e9',
          }}
        >
          {/* Table Header */}
          <div
            className="grid grid-cols-[1fr_1fr_100px_120px_60px] gap-4 px-5 py-3"
            style={{
              borderBottom: '1px solid #dfe6e9',
              background: '#fafbfc',
            }}
          >
            {['Date', 'Version', 'Config', 'TPS', ''].map((h) => (
              <div
                key={h}
                className="text-[13px] font-semibold uppercase tracking-[0.5px]"
                style={{ color: '#636e72' }}
              >
                {h}
              </div>
            ))}
          </div>

          {/* Table Body */}
          {paginatedData.length === 0 ? (
            <div className="px-5 py-12 text-center text-sm" style={{ color: '#636e72' }}>
              No data found for the selected filter.
            </div>
          ) : (
            paginatedData.map((row) => {
              const isExpanded = expandedId === row.id
              const isHighlight = row.n_nodes === 3 && row.n_inout === 1
              return (
                <div key={row.id}>
                  {/* Row */}
                  <button
                    onClick={() => handleToggleExpand(row.id)}
                    className="w-full grid grid-cols-[1fr_1fr_100px_120px_60px] gap-4 px-5 py-4 text-left transition-colors"
                    style={{
                      borderBottom: '1px solid #dfe6e9',
                      background: isExpanded ? '#f1f3f4' : '#ffffff',
                    }}
                    onMouseEnter={(e) => {
                      if (!isExpanded) {
                        e.currentTarget.style.background = '#f1f3f4'
                      }
                    }}
                    onMouseLeave={(e) => {
                      if (!isExpanded) {
                        e.currentTarget.style.background = '#ffffff'
                      }
                    }}
                  >
                    <div className="flex items-center gap-2">
                      <Calendar size={14} style={{ color: '#636e72' }} />
                      <span className="text-sm" style={{ color: '#2d3436' }}>
                        {row.date}
                      </span>
                    </div>
                    <div>
                      <span
                        className="inline-flex items-center px-2.5 py-0.5 rounded-full text-[13px] font-medium"
                        style={{
                          background: '#e8f5e9',
                          color: '#2e7d32',
                        }}
                      >
                        {row.ckb_version_short}
                      </span>
                    </div>
                    <div className="text-sm" style={{ color: '#2d3436' }}>
                      {getConfigLabel(row.n_nodes, row.n_inout)}
                    </div>
                    <div
                      className="font-bold"
                      style={{
                        color: isHighlight ? '#00b894' : '#2d3436',
                        fontSize: isHighlight ? '22px' : '14px',
                        fontWeight: isHighlight ? 700 : 400,
                      }}
                    >
                      {formatNumber(row.transactions_per_second)}
                    </div>
                    <div className="flex items-center justify-center">
                      {isExpanded ? (
                        <ChevronUp size={18} style={{ color: '#636e72' }} />
                      ) : (
                        <ChevronDown size={18} style={{ color: '#636e72' }} />
                      )}
                    </div>
                  </button>

                  {/* Expanded Detail */}
                  {isExpanded && (
                    <div
                      className="px-5 py-5"
                      style={{
                        background: '#fafbfc',
                        borderBottom: '1px solid #dfe6e9',
                        animation: 'expandIn 200ms ease-out',
                      }}
                    >
                      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-x-6 gap-y-4 mb-5">
                        <DetailItem label="Delay Time" value={`${row.delay_time_ms} ms`} />
                        <DetailItem
                          label="Block Range"
                          value={`${formatNumber(row.from_block_number)} - ${formatNumber(row.to_block_number)}`}
                        />
                        <DetailItem
                          label="Tx Size / Second"
                          value={formatNumber(row.transactions_size_per_second)}
                        />
                        <DetailItem
                          label="Avg Block Tx"
                          value={formatNumber(row.average_block_transactions)}
                        />
                        <DetailItem
                          label="Avg Block Tx Size"
                          value={formatNumber(row.average_block_transactions_size)}
                        />
                        <DetailItem
                          label="Avg Block Time"
                          value={`${row.average_block_time_ms} ms`}
                        />
                        <DetailItem
                          label="Total Tx"
                          value={formatNumber(row.total_transactions)}
                        />
                        <DetailItem
                          label="Total Tx Size"
                          value={formatNumber(row.total_transactions_size)}
                        />
                        <DetailItem label="Set Send TPS" value={formatNumber(row.set_send_tps)} />
                        <DetailItem
                          label="Client Send TPS"
                          value={formatNumber(row.client_send_tps)}
                        />
                      </div>

                      <div className="flex flex-wrap gap-3 pt-4" style={{ borderTop: '1px solid #dfe6e9' }}>
                        <a
                          href={row.grafana_link}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors hover:opacity-90"
                          style={{ background: '#0984e3', color: '#ffffff' }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <Activity size={14} />
                          Grafana
                          <ExternalLink size={12} />
                        </a>
                        <a
                          href={row.report_link}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors hover:opacity-90"
                          style={{
                            background: '#ffffff',
                            color: '#0984e3',
                            border: '1px solid #0984e3',
                          }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <ExternalLink size={14} />
                          Report
                          <ExternalLink size={12} />
                        </a>
                      </div>
                    </div>
                  )}
                </div>
              )
            })
          )}
        </div>

        {/* ===== Pagination ===== */}
        {filteredData.length > 0 && (
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mt-5">
            <div className="text-sm" style={{ color: '#636e72' }}>
              Showing {showingFrom} to {showingTo} of {filteredData.length} entries
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={() => handlePageChange(currentPage - 1)}
                disabled={currentPage === 1}
                className="px-3 py-2 rounded-lg text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                style={{
                  background: currentPage === 1 ? '#f1f3f4' : '#ffffff',
                  color: '#2d3436',
                  border: '1px solid #dfe6e9',
                }}
              >
                Previous
              </button>

              {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
                <button
                  key={page}
                  onClick={() => handlePageChange(page)}
                  className="min-w-[36px] h-[36px] flex items-center justify-center rounded-lg text-sm font-medium transition-colors"
                  style={{
                    background: page === currentPage ? '#0984e3' : '#ffffff',
                    color: page === currentPage ? '#ffffff' : '#2d3436',
                    border: '1px solid',
                    borderColor: page === currentPage ? '#0984e3' : '#dfe6e9',
                  }}
                >
                  {page}
                </button>
              ))}

              <button
                onClick={() => handlePageChange(currentPage + 1)}
                disabled={currentPage === totalPages}
                className="px-3 py-2 rounded-lg text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                style={{
                  background: currentPage === totalPages ? '#f1f3f4' : '#ffffff',
                  color: '#2d3436',
                  border: '1px solid #dfe6e9',
                }}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {/* ===== Footer ===== */}
        <div className="mt-10 pt-6 text-center" style={{ borderTop: '1px solid #dfe6e9' }}>
          <a
            href="https://github.com/gpBlockchain/ckb-integration-test/tree/ckb-bench-server/ckb-bench-server#interpretation-of-test-results"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 text-sm font-medium transition-opacity hover:opacity-80"
            style={{ color: '#0984e3' }}
          >
            <ExternalLink size={14} />
            Explanation of Terms
          </a>
        </div>
      </div>

      <style>{`
        @keyframes expandIn {
          from {
            opacity: 0;
            transform: translateY(-4px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }
      `}</style>
    </div>
  )
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[12px] font-medium uppercase tracking-[0.5px] mb-1" style={{ color: '#636e72' }}>
        {label}
      </div>
      <div className="text-sm font-semibold" style={{ color: '#2d3436' }}>
        {value}
      </div>
    </div>
  )
}
