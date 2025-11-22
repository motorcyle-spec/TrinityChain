import React, { useState, useEffect, useCallback } from 'react';
import { Activity, Boxes, Clock, TrendingUp, Award, Coins, Layers, Zap, Database, Target, Search, ChevronDown, ChevronUp, Network, Cpu, BarChart3, Terminal, RefreshCw, Settings, Play, Pause } from 'lucide-react';
import { LineChart, Line, AreaChart, Area, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend } from 'recharts';

const TrinityChainDashboard = () => {
  const [stats, setStats] = useState(null);
  const [blocks, setBlocks] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  // Use empty string for relative URLs (works on Render) or localhost for dev
  const [nodeUrl, setNodeUrl] = useState(window.location.hostname === 'localhost' ? 'http://localhost:3000' : '');
  const [activeTab, setActiveTab] = useState('dashboard');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedBlock, setSelectedBlock] = useState(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [refreshInterval, setRefreshInterval] = useState(3000);
  const [chartData, setChartData] = useState([]);
  const [networkData, setNetworkData] = useState([]);
  const [showSettings, setShowSettings] = useState(false);
  const [expandedBlocks, setExpandedBlocks] = useState(new Set());

  useEffect(() => {
    fetchData();
    if (autoRefresh) {
      const interval = setInterval(fetchData, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [nodeUrl, autoRefresh, refreshInterval]);

  const fetchData = async () => {
    try {
      const [statsRes, blocksRes] = await Promise.all([
        fetch(`${nodeUrl}/api/blockchain/stats`),
        fetch(`${nodeUrl}/api/blockchain/blocks?limit=50`)
      ]);

      if (!statsRes.ok || !blocksRes.ok) {
        throw new Error('Failed to fetch data from node');
      }

      const statsData = await statsRes.json();
      const blocksData = await blocksRes.json();

      setStats(statsData);
      setBlocks(blocksData.blocks || []);

      // Build chart data from recent blocks
      const recentBlocks = (blocksData.blocks || []).slice(0, 20).reverse();
      const newChartData = recentBlocks.map((block, idx) => ({
        block: block.index,
        difficulty: block.difficulty,
        transactions: block.transactions?.length || 0,
        reward: block.reward || 0,
        time: idx
      }));
      setChartData(newChartData);

      // Build network performance data
      if (recentBlocks.length > 1) {
        const networkPerf = recentBlocks.slice(0, 10).map((block, idx) => {
          const prevBlock = recentBlocks[idx + 1];
          const blockTime = prevBlock ?
            (new Date(block.timestamp) - new Date(prevBlock.timestamp)) / 1000 : 0;
          return {
            block: block.index,
            blockTime: Math.max(0, blockTime),
            hashrate: block.difficulty * 1000 / Math.max(blockTime, 0.1)
          };
        }).reverse();
        setNetworkData(networkPerf);
      }

      setError(null);
      setLoading(false);
    } catch (err) {
      setError(err.message);
      setLoading(false);
    }
  };

  const formatNumber = (num) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(2)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(2)}K`;
    return new Intl.NumberFormat('en-US').format(num);
  };

  const formatFullNumber = (num) => {
    return new Intl.NumberFormat('en-US').format(num);
  };

  const formatTime = (seconds) => {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (mins < 60) return `${mins}m ${secs}s`;
    const hours = Math.floor(mins / 60);
    const remainMins = mins % 60;
    return `${hours}h ${remainMins}m`;
  };

  const formatHash = (hash) => {
    if (!hash) return 'N/A';
    return `${hash.slice(0, 12)}...${hash.slice(-10)}`;
  };

  const calculatePercentage = (current, total) => {
    return ((current / total) * 100).toFixed(3);
  };

  const calculateHashrate = () => {
    if (!stats || !stats.difficulty || !stats.avgBlockTime) return 0;
    return (stats.difficulty * 1000 / Math.max(stats.avgBlockTime, 0.1)).toFixed(2);
  };

  const toggleBlockExpansion = (blockIndex) => {
    const newExpanded = new Set(expandedBlocks);
    if (newExpanded.has(blockIndex)) {
      newExpanded.delete(blockIndex);
    } else {
      newExpanded.add(blockIndex);
    }
    setExpandedBlocks(newExpanded);
  };

  const filteredBlocks = blocks.filter(block =>
    searchQuery === '' ||
    block.hash?.toLowerCase().includes(searchQuery.toLowerCase()) ||
    block.index?.toString().includes(searchQuery) ||
    block.previousHash?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (loading && !stats) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-950 via-purple-950 to-slate-950 flex items-center justify-center">
        <div className="text-center">
          <div className="relative w-24 h-24 mx-auto mb-6">
            <div className="absolute inset-0 border-4 border-purple-500/30 rounded-full"></div>
            <div className="absolute inset-0 border-4 border-transparent border-t-purple-500 rounded-full animate-spin"></div>
            <img src="/logo.png" alt="TrinityChain" className="absolute inset-0 m-auto w-12 h-12 object-contain" />
          </div>
          <p className="text-purple-200 text-xl font-semibold">Connecting to TrinityChain Node...</p>
          <p className="text-purple-400 text-sm mt-2">{nodeUrl}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-purple-950 to-slate-950 text-white">
      {/* Top Navigation Bar */}
      <div className="bg-slate-900/80 backdrop-blur-xl border-b border-purple-500/20 sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-3">
                <div className="relative">
                  <img src="/logo.png" alt="TrinityChain" className="w-10 h-10 object-contain" />
                  <div className="absolute -top-1 -right-1 w-3 h-3 bg-green-500 rounded-full border-2 border-slate-900 animate-pulse"></div>
                </div>
                <div>
                  <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 via-pink-400 to-purple-400 bg-clip-text text-transparent">
                    TrinityChain
                  </h1>
                  <p className="text-xs text-purple-300">Mining Dashboard v0.2.0</p>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={() => setAutoRefresh(!autoRefresh)}
                className={`p-2 rounded-lg transition-all ${autoRefresh ? 'bg-green-600 hover:bg-green-700' : 'bg-slate-700 hover:bg-slate-600'}`}
                title={autoRefresh ? 'Pause auto-refresh' : 'Resume auto-refresh'}
              >
                {autoRefresh ? <Pause size={18} /> : <Play size={18} />}
              </button>
              <button
                onClick={fetchData}
                className="p-2 rounded-lg bg-purple-600 hover:bg-purple-700 transition-all"
                title="Manual Refresh"
              >
                <RefreshCw size={18} className={loading ? 'animate-spin' : ''} />
              </button>
              <button
                onClick={() => setShowSettings(!showSettings)}
                className="p-2 rounded-lg bg-slate-700 hover:bg-slate-600 transition-all"
                title="Settings"
              >
                <Settings size={18} />
              </button>
              <div className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg">
                <div className={`w-2 h-2 rounded-full ${error ? 'bg-red-500' : 'bg-green-500'} animate-pulse`}></div>
                <span className="text-xs text-purple-200">{error ? 'Offline' : 'Live'}</span>
              </div>
            </div>
          </div>

          {/* Settings Panel */}
          {showSettings && (
            <div className="mt-4 p-4 bg-slate-800/50 rounded-lg border border-purple-500/20">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="text-sm text-purple-200 mb-2 block">Node URL</label>
                  <input
                    type="text"
                    value={nodeUrl}
                    onChange={(e) => setNodeUrl(e.target.value)}
                    className="w-full bg-slate-900/50 border border-purple-500/30 rounded px-4 py-2 text-white focus:outline-none focus:border-purple-400"
                    placeholder="http://localhost:3000"
                  />
                </div>
                <div>
                  <label className="text-sm text-purple-200 mb-2 block">Refresh Interval (ms)</label>
                  <input
                    type="number"
                    value={refreshInterval}
                    onChange={(e) => setRefreshInterval(parseInt(e.target.value) || 3000)}
                    className="w-full bg-slate-900/50 border border-purple-500/30 rounded px-4 py-2 text-white focus:outline-none focus:border-purple-400"
                    min="1000"
                    step="1000"
                  />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="max-w-7xl mx-auto p-6">
        {error && (
          <div className="bg-red-900/30 border-l-4 border-red-500 rounded-lg p-4 mb-6 backdrop-blur-sm">
            <div className="flex items-center gap-3">
              <div className="bg-red-500/20 rounded-full p-2">
                <Activity className="text-red-400" size={20} />
              </div>
              <div>
                <p className="text-red-200 font-semibold">Connection Error</p>
                <p className="text-red-300 text-sm">{error} - Ensure TrinityChain node is running on {nodeUrl}</p>
              </div>
            </div>
          </div>
        )}

        {/* Tabs */}
        <div className="flex gap-2 mb-6 overflow-x-auto pb-2">
          {[
            { id: 'dashboard', label: 'Dashboard', icon: BarChart3 },
            { id: 'analytics', label: 'Analytics', icon: Activity },
            { id: 'explorer', label: 'Block Explorer', icon: Boxes },
            { id: 'network', label: 'Network', icon: Network }
          ].map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-6 py-3 rounded-lg font-semibold transition-all whitespace-nowrap ${
                activeTab === tab.id
                  ? 'bg-gradient-to-r from-purple-600 to-pink-600 text-white shadow-lg shadow-purple-500/50'
                  : 'bg-slate-800/50 text-purple-200 hover:bg-slate-800 border border-purple-500/20'
              }`}
            >
              <tab.icon size={18} />
              {tab.label}
            </button>
          ))}
        </div>

        {activeTab === 'dashboard' && stats && (
          <>
            {/* Hero Stats */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
              <HeroStatCard
                icon={<Boxes className="text-purple-400" size={28} />}
                label="Chain Height"
                value={formatFullNumber(stats.chainHeight || 0)}
                subtext={`${formatNumber(stats.blocksMined || 0)} mined`}
                gradient="from-purple-600 to-purple-800"
              />
              <HeroStatCard
                icon={<Zap className="text-yellow-400" size={28} />}
                label="Hashrate"
                value={`${calculateHashrate()} H/s`}
                subtext={`Difficulty: ${stats.difficulty || 0}`}
                gradient="from-yellow-600 to-orange-600"
              />
              <HeroStatCard
                icon={<Coins className="text-green-400" size={28} />}
                label="Total Earned"
                value={formatNumber(stats.totalEarned || 0)}
                subtext={`${formatNumber(stats.currentReward || 0)} per block`}
                gradient="from-green-600 to-emerald-600"
              />
              <HeroStatCard
                icon={<Clock className="text-blue-400" size={28} />}
                label="Block Time"
                value={`${(stats.avgBlockTime || 0).toFixed(2)}s`}
                subtext={`Uptime: ${formatTime(stats.uptime || 0)}`}
                gradient="from-blue-600 to-cyan-600"
              />
            </div>

            {/* Supply and Halving */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
              <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div className="bg-purple-600/20 rounded-lg p-3">
                      <Database className="text-purple-400" size={24} />
                    </div>
                    <div>
                      <h3 className="text-xl font-bold">Token Supply</h3>
                      <p className="text-purple-300 text-sm">Distribution Progress</p>
                    </div>
                  </div>
                </div>
                <div className="space-y-4">
                  <div className="flex justify-between items-end">
                    <div>
                      <p className="text-purple-300 text-sm">Current Supply</p>
                      <p className="text-3xl font-bold">{formatNumber(stats.totalSupply || 0)}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-purple-300 text-sm">Max Supply</p>
                      <p className="text-2xl font-bold">{formatNumber(stats.maxSupply || 420000000)}</p>
                    </div>
                  </div>
                  <div className="relative">
                    <div className="w-full bg-slate-800 rounded-full h-6 overflow-hidden">
                      <div
                        className="h-6 rounded-full bg-gradient-to-r from-purple-500 via-pink-500 to-purple-500 transition-all duration-1000 flex items-center justify-end pr-3"
                        style={{ width: `${Math.min(100, calculatePercentage(stats.totalSupply || 0, stats.maxSupply || 420000000))}%` }}
                      >
                        <span className="text-xs font-bold text-white drop-shadow-lg">
                          {calculatePercentage(stats.totalSupply || 0, stats.maxSupply || 420000000)}%
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="grid grid-cols-3 gap-3 pt-2">
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-purple-300 text-xs mb-1">Remaining</p>
                      <p className="font-bold text-sm">{formatNumber((stats.maxSupply || 420000000) - (stats.totalSupply || 0))}</p>
                    </div>
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-purple-300 text-xs mb-1">Circulating</p>
                      <p className="font-bold text-sm">{formatNumber(stats.totalSupply || 0)}</p>
                    </div>
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-purple-300 text-xs mb-1">Burned</p>
                      <p className="font-bold text-sm">0</p>
                    </div>
                  </div>
                </div>
              </div>

              <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-pink-500/20 shadow-xl">
                <div className="flex items-center justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div className="bg-pink-600/20 rounded-lg p-3">
                      <Award className="text-pink-400" size={24} />
                    </div>
                    <div>
                      <h3 className="text-xl font-bold">Halving Schedule</h3>
                      <p className="text-pink-300 text-sm">Reward Reduction</p>
                    </div>
                  </div>
                </div>
                <div className="space-y-4">
                  <div className="flex justify-between items-end">
                    <div>
                      <p className="text-pink-300 text-sm">Current Era</p>
                      <p className="text-5xl font-bold bg-gradient-to-r from-pink-400 to-purple-400 bg-clip-text text-transparent">
                        {stats.halvingEra || 0}
                      </p>
                    </div>
                    <div className="text-right">
                      <p className="text-pink-300 text-sm">Current Reward</p>
                      <p className="text-3xl font-bold text-green-400">{formatNumber(stats.currentReward || 0)}</p>
                    </div>
                  </div>
                  <div className="relative">
                    <div className="w-full bg-slate-800 rounded-full h-6 overflow-hidden">
                      <div
                        className="h-6 rounded-full bg-gradient-to-r from-pink-500 via-purple-500 to-pink-500 transition-all duration-1000 flex items-center justify-end pr-3"
                        style={{
                          width: `${Math.min(100, Math.max(0, 100 - ((stats.blocksToHalving || 0) / 210000 * 100)))}%`
                        }}
                      >
                        <span className="text-xs font-bold text-white drop-shadow-lg">
                          {Math.max(0, 100 - ((stats.blocksToHalving || 0) / 210000 * 100)).toFixed(1)}%
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="grid grid-cols-3 gap-3 pt-2">
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-pink-300 text-xs mb-1">Blocks Left</p>
                      <p className="font-bold text-sm">{formatNumber(stats.blocksToHalving || 0)}</p>
                    </div>
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-pink-300 text-xs mb-1">Next Block</p>
                      <p className="font-bold text-sm">{formatNumber((stats.chainHeight || 0) + (stats.blocksToHalving || 0))}</p>
                    </div>
                    <div className="bg-slate-800/50 rounded-lg p-3 text-center">
                      <p className="text-pink-300 text-xs mb-1">Next Reward</p>
                      <p className="font-bold text-sm">{formatNumber((stats.currentReward || 1000) / 2)}</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Recent Blocks */}
            <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="bg-purple-600/20 rounded-lg p-3">
                    <Boxes className="text-purple-400" size={24} />
                  </div>
                  <div>
                    <h3 className="text-xl font-bold">Latest Blocks</h3>
                    <p className="text-purple-300 text-sm">Most recent mining activity</p>
                  </div>
                </div>
              </div>
              <div className="space-y-2">
                {blocks.slice(0, 10).map((block, idx) => (
                  <div
                    key={idx}
                    className="bg-slate-800/50 rounded-lg p-4 hover:bg-slate-800/70 transition-all border border-purple-500/10 hover:border-purple-500/30"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div className="bg-purple-600/20 rounded-lg px-3 py-2">
                          <span className="text-purple-400 font-mono font-bold text-lg">#{block.index}</span>
                        </div>
                        <div>
                          <p className="text-sm text-purple-200 font-mono">{formatHash(block.hash)}</p>
                          <p className="text-xs text-purple-400 mt-1">{new Date(block.timestamp).toLocaleString()}</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-6">
                        <div className="text-right">
                          <p className="text-xs text-purple-300">Transactions</p>
                          <p className="font-bold">{block.transactions?.length || 0}</p>
                        </div>
                        <div className="text-right">
                          <p className="text-xs text-purple-300">Difficulty</p>
                          <p className="font-bold">{block.difficulty}</p>
                        </div>
                        <div className="bg-green-600/20 rounded-lg px-3 py-2">
                          <p className="text-green-400 font-bold">+{formatNumber(block.reward || 0)}</p>
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        {activeTab === 'analytics' && (
          <>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
              <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
                <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                  <TrendingUp className="text-purple-400" />
                  Difficulty Trend
                </h3>
                <ResponsiveContainer width="100%" height={250}>
                  <AreaChart data={chartData}>
                    <defs>
                      <linearGradient id="difficultyGradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#a855f7" stopOpacity={0.8}/>
                        <stop offset="95%" stopColor="#a855f7" stopOpacity={0}/>
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                    <XAxis dataKey="block" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                    <YAxis stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                    <Tooltip
                      contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #6366f1', borderRadius: '8px' }}
                      labelStyle={{ color: '#e2e8f0' }}
                    />
                    <Area type="monotone" dataKey="difficulty" stroke="#a855f7" fillOpacity={1} fill="url(#difficultyGradient)" />
                  </AreaChart>
                </ResponsiveContainer>
              </div>

              <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
                <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                  <Activity className="text-green-400" />
                  Transaction Activity
                </h3>
                <ResponsiveContainer width="100%" height={250}>
                  <BarChart data={chartData}>
                    <defs>
                      <linearGradient id="txGradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#10b981" stopOpacity={0.8}/>
                        <stop offset="95%" stopColor="#10b981" stopOpacity={0.3}/>
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                    <XAxis dataKey="block" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                    <YAxis stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                    <Tooltip
                      contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #10b981', borderRadius: '8px' }}
                      labelStyle={{ color: '#e2e8f0' }}
                    />
                    <Bar dataKey="transactions" fill="url(#txGradient)" radius={[8, 8, 0, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>

            <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <Award className="text-yellow-400" />
                Block Rewards
              </h3>
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={chartData}>
                  <defs>
                    <linearGradient id="rewardGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#eab308" stopOpacity={0.8}/>
                      <stop offset="95%" stopColor="#eab308" stopOpacity={0}/>
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                  <XAxis dataKey="block" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                  <YAxis stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #eab308', borderRadius: '8px' }}
                    labelStyle={{ color: '#e2e8f0' }}
                  />
                  <Line type="monotone" dataKey="reward" stroke="#eab308" strokeWidth={3} dot={{ fill: '#eab308', r: 4 }} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </>
        )}

        {activeTab === 'network' && (
          <>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-6">
              <StatCard
                icon={<Cpu className="text-cyan-400" />}
                label="Network Hashrate"
                value={`${calculateHashrate()} H/s`}
                trend="+12.5%"
                color="cyan"
              />
              <StatCard
                icon={<Network className="text-blue-400" />}
                label="Active Nodes"
                value="1"
                trend="Stable"
                color="blue"
              />
              <StatCard
                icon={<Target className="text-red-400" />}
                label="Network Difficulty"
                value={formatNumber(stats?.difficulty || 0)}
                trend={`Target: ${stats?.difficulty || 0}`}
                color="red"
              />
            </div>

            <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl mb-6">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <Activity className="text-purple-400" />
                Network Performance
              </h3>
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={networkData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                  <XAxis dataKey="block" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                  <YAxis yAxisId="left" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                  <YAxis yAxisId="right" orientation="right" stroke="#94a3b8" tick={{ fill: '#94a3b8' }} />
                  <Tooltip
                    contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #6366f1', borderRadius: '8px' }}
                    labelStyle={{ color: '#e2e8f0' }}
                  />
                  <Legend />
                  <Line yAxisId="left" type="monotone" dataKey="blockTime" stroke="#10b981" strokeWidth={2} name="Block Time (s)" />
                  <Line yAxisId="right" type="monotone" dataKey="hashrate" stroke="#a855f7" strokeWidth={2} name="Hashrate" />
                </LineChart>
              </ResponsiveContainer>
            </div>

            <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl">
              <h3 className="text-xl font-bold mb-4 flex items-center gap-2">
                <Terminal className="text-green-400" />
                Node Information
              </h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <InfoRow label="Node URL" value={nodeUrl} />
                <InfoRow label="Protocol Version" value="1.0.0" />
                <InfoRow label="Network ID" value="TrinityChain" />
                <InfoRow label="Sync Status" value="Synchronized" />
                <InfoRow label="Peer Count" value="1" />
                <InfoRow label="Latest Block" value={`#${stats?.chainHeight || 0}`} />
              </div>
            </div>
          </>
        )}

        {activeTab === 'explorer' && (
          <>
            <div className="bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border border-purple-500/20 shadow-xl mb-6">
              <div className="flex items-center gap-4 mb-4">
                <div className="flex-1 relative">
                  <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-purple-400" size={20} />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="Search by block height, hash, or previous hash..."
                    className="w-full bg-slate-800/50 border border-purple-500/30 rounded-lg pl-10 pr-4 py-3 text-white placeholder-purple-300/50 focus:outline-none focus:border-purple-400"
                  />
                </div>
                <div className="bg-slate-800/50 border border-purple-500/20 rounded-lg px-4 py-3">
                  <span className="text-purple-200 text-sm">
                    {filteredBlocks.length} blocks
                  </span>
                </div>
              </div>
            </div>

            <div className="space-y-4">
              {filteredBlocks.map((block, idx) => (
                <div
                  key={idx}
                  className="bg-slate-900/50 backdrop-blur-xl rounded-2xl border border-purple-500/20 shadow-xl overflow-hidden hover:border-purple-500/40 transition-all"
                >
                  <div
                    className="p-6 cursor-pointer"
                    onClick={() => toggleBlockExpansion(block.index)}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div className="bg-gradient-to-br from-purple-600 to-pink-600 rounded-xl px-4 py-3 shadow-lg">
                          <span className="text-white font-mono font-bold text-xl">#{block.index}</span>
                        </div>
                        <div>
                          <p className="text-purple-200 font-mono text-sm mb-1">{formatHash(block.hash)}</p>
                          <p className="text-purple-400 text-xs">{new Date(block.timestamp).toLocaleString()}</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-6">
                        <div className="text-center">
                          <p className="text-purple-300 text-xs mb-1">Difficulty</p>
                          <p className="font-bold text-lg">{block.difficulty}</p>
                        </div>
                        <div className="text-center">
                          <p className="text-purple-300 text-xs mb-1">Nonce</p>
                          <p className="font-bold text-lg">{block.nonce}</p>
                        </div>
                        <div className="text-center">
                          <p className="text-purple-300 text-xs mb-1">Transactions</p>
                          <p className="font-bold text-lg">{block.transactions?.length || 0}</p>
                        </div>
                        <div className="bg-green-600/20 rounded-lg px-4 py-2 border border-green-500/30">
                          <p className="text-green-400 font-bold text-lg">+{formatNumber(block.reward || 0)}</p>
                        </div>
                        <button className="text-purple-400 hover:text-purple-300 transition-colors">
                          {expandedBlocks.has(block.index) ? <ChevronUp size={24} /> : <ChevronDown size={24} />}
                        </button>
                      </div>
                    </div>
                  </div>

                  {expandedBlocks.has(block.index) && (
                    <div className="border-t border-purple-500/20 bg-slate-950/50 p-6">
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div>
                          <h4 className="text-purple-300 text-sm font-semibold mb-3 flex items-center gap-2">
                            <Database size={16} />
                            Block Details
                          </h4>
                          <div className="space-y-3">
                            <DetailRow label="Block Height" value={`#${block.index}`} />
                            <DetailRow label="Timestamp" value={new Date(block.timestamp).toISOString()} />
                            <DetailRow label="Difficulty" value={block.difficulty} />
                            <DetailRow label="Nonce" value={block.nonce} />
                            <DetailRow label="Reward" value={`${formatFullNumber(block.reward || 0)} TRC`} />
                            <DetailRow label="Size" value={`${JSON.stringify(block).length} bytes`} />
                          </div>
                        </div>
                        <div>
                          <h4 className="text-purple-300 text-sm font-semibold mb-3 flex items-center gap-2">
                            <Boxes size={16} />
                            Hash Information
                          </h4>
                          <div className="space-y-3">
                            <div>
                              <p className="text-purple-400 text-xs mb-1">Block Hash</p>
                              <p className="font-mono text-xs bg-slate-900/50 p-2 rounded break-all border border-purple-500/20">
                                {block.hash}
                              </p>
                            </div>
                            <div>
                              <p className="text-purple-400 text-xs mb-1">Previous Hash</p>
                              <p className="font-mono text-xs bg-slate-900/50 p-2 rounded break-all border border-purple-500/20">
                                {block.previousHash}
                              </p>
                            </div>
                          </div>
                        </div>
                      </div>

                      {block.transactions && block.transactions.length > 0 && (
                        <div className="mt-6">
                          <h4 className="text-purple-300 text-sm font-semibold mb-3 flex items-center gap-2">
                            <Activity size={16} />
                            Transactions ({block.transactions.length})
                          </h4>
                          <div className="space-y-2">
                            {block.transactions.map((tx, txIdx) => (
                              <div key={txIdx} className="bg-slate-900/50 rounded-lg p-3 border border-purple-500/10">
                                <div className="flex items-center justify-between">
                                  <p className="font-mono text-sm text-purple-200">
                                    {tx.hash ? formatHash(tx.hash) : `Transaction #${txIdx + 1}`}
                                  </p>
                                  <div className="flex items-center gap-4 text-xs">
                                    {tx.from && <span className="text-purple-400">From: {formatHash(tx.from)}</span>}
                                    {tx.to && <span className="text-purple-400">To: {formatHash(tx.to)}</span>}
                                    {tx.amount && <span className="text-green-400 font-bold">+{tx.amount}</span>}
                                  </div>
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
};

const HeroStatCard = ({ icon, label, value, subtext, gradient }) => (
  <div className={`bg-gradient-to-br ${gradient} rounded-2xl p-6 shadow-xl border border-white/10 transform hover:scale-105 transition-all`}>
    <div className="flex items-start justify-between mb-3">
      <div className="bg-white/10 backdrop-blur-sm rounded-xl p-3">
        {icon}
      </div>
    </div>
    <p className="text-white/80 text-sm mb-1">{label}</p>
    <p className="text-3xl font-bold text-white mb-1">{value}</p>
    <p className="text-white/60 text-xs">{subtext}</p>
  </div>
);

const StatCard = ({ icon, label, value, trend, color }) => {
  const colorClasses = {
    cyan: 'border-cyan-500/20 hover:border-cyan-500/40',
    blue: 'border-blue-500/20 hover:border-blue-500/40',
    red: 'border-red-500/20 hover:border-red-500/40'
  };

  return (
    <div className={`bg-slate-900/50 backdrop-blur-xl rounded-2xl p-6 border ${colorClasses[color]} shadow-xl transition-all hover:scale-105`}>
      <div className="flex items-center gap-3 mb-4">
        <div className={`bg-${color}-600/20 rounded-lg p-3`}>
          {icon}
        </div>
        <span className="text-purple-200 text-sm font-medium">{label}</span>
      </div>
      <div className="text-4xl font-bold mb-2">{value}</div>
      <div className="text-sm text-purple-400">{trend}</div>
    </div>
  );
};

const InfoRow = ({ label, value }) => (
  <div className="bg-slate-800/50 rounded-lg p-3 border border-purple-500/10">
    <p className="text-purple-400 text-xs mb-1">{label}</p>
    <p className="font-mono text-sm text-white">{value}</p>
  </div>
);

const DetailRow = ({ label, value }) => (
  <div className="flex justify-between items-center">
    <span className="text-purple-400 text-sm">{label}</span>
    <span className="font-mono text-sm text-white">{value}</span>
  </div>
);

export default TrinityChainDashboard;
