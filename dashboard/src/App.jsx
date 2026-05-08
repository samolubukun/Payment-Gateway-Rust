import React, { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  ShoppingCart, 
  ShieldCheck, 
  CreditCard, 
  RefreshCw, 
  Play,
  Terminal,
  Activity,
  Server,
  ChevronRight
} from 'lucide-react';
import axios from 'axios';

const GATEWAY_URL = 'http://localhost:3000';
const BANK_URL = 'http://localhost:8787';

const GithubIcon = ({ size = 16 }) => (
  <svg 
    width={size} 
    height={size} 
    viewBox="0 0 24 24" 
    fill="none" 
    stroke="currentColor" 
    strokeWidth="2" 
    strokeLinecap="round" 
    strokeLinejoin="round"
  >
    <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7a3.37 3.37 0 0 0-.94 2.58V22"></path>
  </svg>
);

function App() {
  const [logs, setLogs] = useState([]);
  const [traffic, setTraffic] = useState([]);
  const [isSimulating, setIsSimulating] = useState(false);
  const [step, setStep] = useState(0); 
  const [chaosConfig, setChaosConfig] = useState({
    failureRate: 0.1,
    latency: 500
  });
  const [gatewayStatus, setGatewayStatus] = useState('offline');
  const [bankStatus, setBankStatus] = useState('offline');
  const [simulationId, setSimulationId] = useState(0);

  const addLog = useCallback((msg, type = 'info') => {
    setLogs(prev => [{ id: Date.now() + Math.random(), msg, type, time: new Date().toLocaleTimeString() }, ...prev].slice(0, 100));
  }, []);

  const addTraffic = useCallback((method, path, body, response = null) => {
    setTraffic(prev => [{ id: Date.now() + Math.random(), method, path, body, response, time: new Date().toLocaleTimeString() }, ...prev].slice(0, 50));
  }, []);

  const checkHealth = async () => {
    try {
      await axios.get(`${GATEWAY_URL}/health`);
      setGatewayStatus('online');
    } catch {
      setGatewayStatus('offline');
    }
    try {
      await axios.get(`${BANK_URL}/docs`);
      setBankStatus('online');
    } catch {
      setBankStatus('offline');
    }
  };

  useEffect(() => {
    checkHealth();
    const interval = setInterval(checkHealth, 3000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    const syncChaos = async () => {
      try {
        await axios.post(`${BANK_URL}/api/v1/chaos`, {
          enabled: true,
          failure_rate: chaosConfig.failureRate,
          min_latency_ms: chaosConfig.latency,
          max_latency_ms: chaosConfig.latency + 200
        });
        addLog(`System parameters synchronized. Latency: ${chaosConfig.latency}ms`, 'info');
      } catch (err) {
        console.error('Sync failed', err);
      }
    };
    const timeout = setTimeout(syncChaos, 500);
    return () => clearTimeout(timeout);
  }, [chaosConfig, addLog]);

  const runSimulation = async () => {
    if (isSimulating) return;
    
    // Reset for new run
    const currentId = Date.now();
    setSimulationId(currentId);
    setIsSimulating(true);
    setStep(1);
    
    addLog(`INIT: Simulation sequence ${currentId.toString().slice(-6)} started.`, 'info');

    const paymentData = {
      order_id: `ORD-${Math.floor(Math.random() * 1000000)}`,
      customer_id: `CUST-${Math.floor(Math.random() * 1000)}`,
      amount_cents: Math.floor(Math.random() * 5000) + 100,
      currency: 'USD',
      card: {
        number: '4111111111111111',
        exp_month: 12,
        exp_year: 2030,
        cvv: '123'
      }
    };

    let step2Timer, step3Timer;

    try {
      addTraffic('POST', '/v1/payments', paymentData);
      addLog(`REQ: Sending authorization payload to Gateway...`, 'info');
      
      step2Timer = setTimeout(() => {
        setStep(2);
        addLog('PROX: Gateway forwarding upstream to Mock Bank...', 'info');
        addTraffic('POST', '/api/v1/authorizations', paymentData);
      }, 1000);

      step3Timer = setTimeout(() => {
        setStep(3);
        addLog('EXEC: Mock Bank processing core logic...', 'info');
      }, 2200);

      const response = await axios.post(`${GATEWAY_URL}/v1/payments`, paymentData, {
        headers: {
          'Idempotency-Key': `sim-${currentId}-${Math.floor(Math.random() * 1000)}`,
          'X-Merchant-Id': 'simulation_merchant'
        }
      });
      
      // If we got here, success
      clearTimeout(step2Timer);
      clearTimeout(step3Timer);
      
      setStep(4);
      addLog('RES: Bank response received by Gateway node.', 'success');
      
      setTimeout(() => {
        setStep(5);
        addLog(`DONE: Payment authorized. Transaction ID: ${response.data.id}`, 'success');
        addTraffic('RESPONSE', '201 Created', null, response.data);
      }, 800);

    } catch (err) {
      clearTimeout(step2Timer);
      clearTimeout(step3Timer);
      setStep(0);
      const errorMsg = err.response?.data?.error?.message || err.message;
      addLog(`FAIL: ${errorMsg}`, 'error');
      addTraffic('ERROR', err.response?.status || '500', null, err.response?.data);
    } finally {
      setTimeout(() => {
        setIsSimulating(false);
        setStep(0);
      }, 4000);
    }
  };

  return (
    <div className="app-wrapper">
      <header>
        <div className="brand">
          <CreditCard size={24} color="#ffffff" />
          <h1>FICPAY <span style={{ opacity: 0.8, fontWeight: 300 }}>SIMULATOR</span></h1>
        </div>
        <a 
          href="https://github.com/samolubukun/Payment-Gateway-Rust" 
          target="_blank" 
          rel="noopener noreferrer" 
          className="github-link"
        >
          <GithubIcon size={16} />
          <span>SOURCE CODE</span>
        </a>

        <div className="system-status">
          <div className={`status-item ${gatewayStatus === 'online' ? 'status-online' : 'status-offline'}`}>
            <div className="status-dot"></div>
            GATEWAY NODE
          </div>
          <div className={`status-item ${bankStatus === 'online' ? 'status-online' : 'status-offline'}`}>
            <div className="status-dot"></div>
            CORE BANK
          </div>
        </div>
      </header>

      <div className="main-simulation-area">
        <div className="stage-panel">
          <div className="node-container">
            {/* Dashboard */}
            <div className={`node ${step === 1 || step === 5 ? 'active' : ''}`} style={{ position: 'absolute', left: '4rem' }}>
              <ShoppingCart className="node-icon" size={32} />
              <span className="node-label">FICMART</span>
              {(step === 1 || step === 5) && <div className="traffic-label">{step === 1 ? 'POST /v1/payments' : '201 AUTHORIZED'}</div>}
            </div>

            {/* Connection 1 */}
            <div className="connection" style={{ left: '110px', width: 'calc(50% - 180px)' }}></div>
            {step === 1 && (
              <motion.div 
                className="packet"
                initial={{ left: '110px', top: '50%', y: '-50%' }}
                animate={{ left: 'calc(50% - 60px)' }}
                transition={{ duration: 1, ease: "easeInOut" }}
              />
            )}
            {step === 5 && (
              <motion.div 
                className="packet"
                initial={{ left: 'calc(50% - 60px)', top: '50%', y: '-50%' }}
                animate={{ left: '110px' }}
                transition={{ duration: 1, ease: "easeInOut" }}
              />
            )}

            {/* Gateway */}
            <div className={`node ${step === 2 || step === 4 ? 'active' : ''}`} style={{ position: 'absolute', left: '50%', transform: 'translateX(-50%)' }}>
              <ShieldCheck className="node-icon" size={32} />
              <span className="node-label">GATEWAY</span>
              {(step === 2 || step === 4) && <div className="traffic-label" style={{ top: '-40px' }}>{step === 2 ? 'FWD /authorizations' : 'HTTP 200 OK'}</div>}
            </div>

            {/* Connection 2 */}
            <div className="connection" style={{ right: '110px', width: 'calc(50% - 180px)' }}></div>
            {step === 2 && (
              <motion.div 
                className="packet"
                initial={{ left: 'calc(50% + 60px)', top: '50%', y: '-50%' }}
                animate={{ left: 'calc(100% - 110px)' }}
                transition={{ duration: 1, ease: "easeInOut" }}
              />
            )}
            {step === 4 && (
              <motion.div 
                className="packet"
                initial={{ left: 'calc(100% - 110px)', top: '50%', y: '-50%' }}
                animate={{ left: 'calc(50% + 60px)' }}
                transition={{ duration: 1, ease: "easeInOut" }}
              />
            )}

            {/* Bank */}
            <div className={`node ${step === 3 ? 'active' : ''}`} style={{ position: 'absolute', right: '4rem' }}>
              <Server className="node-icon" size={40} />
              <span className="node-label">BANK CORE</span>
              {step === 3 && <div className="traffic-label">PROCESSING...</div>}
            </div>
          </div>

          <div className="console-panel">
            <div className="console-column">
              <div className="console-header">
                <span><Activity size={12} style={{ marginRight: 4 }} /> Traffic Monitor</span>
                <span style={{ opacity: 0.5 }}>Real-time I/O</span>
              </div>
              <div className="console-content">
                {traffic.map(t => (
                  <div key={t.id} className="traffic-entry">
                    <div>
                      <span className="traffic-method">{t.method}</span>{' '}
                      <span className="traffic-path">{t.path}</span>
                    </div>
                    {t.body && <div style={{ fontSize: '0.7rem', opacity: 0.6, marginTop: 4 }}>BODY: {JSON.stringify(t.body)}</div>}
                    {t.response && <div style={{ fontSize: '0.7rem', color: '#22c55e', marginTop: 4 }}>RES: {JSON.stringify(t.response)}</div>}
                  </div>
                ))}
                {traffic.length === 0 && <div style={{ opacity: 0.3, textAlign: 'center', marginTop: '2rem' }}>Listening for traffic...</div>}
              </div>
            </div>
            <div className="console-column">
              <div className="console-header">
                <span><Terminal size={12} style={{ marginRight: 4 }} /> Event Timeline</span>
                <span style={{ opacity: 0.5 }}>System Logs</span>
              </div>
              <div className="console-content">
                {logs.map(log => (
                  <div key={log.id} className="log-entry">
                    <span className="log-time">[{log.time}]</span>
                    <span className={`log-msg log-${log.type}`}>{log.msg}</span>
                  </div>
                ))}
                {logs.length === 0 && <div style={{ opacity: 0.3, textAlign: 'center', marginTop: '2rem' }}>Waiting for sequence start...</div>}
              </div>
            </div>
          </div>
        </div>

        <div className="control-panel">
          <div className="control-group">
            <h3>SIMULATION ENGINE</h3>
            <div className="slider-container">
              <div className="slider-header">
                <label>NETWORK LATENCY</label>
                <span>{chaosConfig.latency}ms</span>
              </div>
              <input 
                type="range" min="0" max="10000" step="100" 
                value={chaosConfig.latency} 
                onChange={(e) => setChaosConfig({...chaosConfig, latency: parseInt(e.target.value)})}
              />
            </div>
            <div className="slider-container">
              <div className="slider-header">
                <label>FAILURE PROBABILITY</label>
                <span>{(chaosConfig.failureRate * 100).toFixed(0)}%</span>
              </div>
              <input 
                type="range" min="0" max="1" step="0.05" 
                value={chaosConfig.failureRate} 
                onChange={(e) => setChaosConfig({...chaosConfig, failureRate: parseFloat(e.target.value)})}
              />
            </div>
            
            <button 
              className="btn-start" 
              onClick={runSimulation}
              disabled={isSimulating || gatewayStatus === 'offline'}
            >
              {isSimulating ? (
                <><RefreshCw className="animate-spin" size={20} /> PROCESSING SEQUENCE...</>
              ) : (
                <><Play size={20} /> EXECUTE SIMULATION</>
              )}
            </button>
          </div>

          <div className="control-group" style={{ flex: 1 }}>
            <h3>SYSTEM METRICS</h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div style={{ padding: '1rem', background: 'rgba(0,0,0,0.2)', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.05)' }}>
                <div style={{ fontSize: '0.65rem', color: '#64748b', marginBottom: '0.25rem' }}>UPTIME</div>
                <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#22c55e' }}>99.998%</div>
              </div>
              <div style={{ padding: '1rem', background: 'rgba(0,0,0,0.2)', borderRadius: '12px', border: '1px solid rgba(255,255,255,0.05)' }}>
                <div style={{ fontSize: '0.65rem', color: '#64748b', marginBottom: '0.25rem' }}>AVG. RESPONSE TIME</div>
                <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#818cf8' }}>{chaosConfig.latency + 120}ms</div>
              </div>
            </div>
            <p style={{ fontSize: '0.7rem', color: '#475569', marginTop: '1.5rem', fontStyle: 'italic' }}>
              Simulator v1.2.0 - Running on production environment logic. Use parameters to test system resilience.
            </p>
          </div>
        </div>
      </div>

      <style>{`
        .animate-spin {
          animation: spin 1s linear infinite;
        }
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}

export default App;
