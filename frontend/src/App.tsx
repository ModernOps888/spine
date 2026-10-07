import { useState, useEffect, useRef } from 'react';
import { TopNav } from './components/TopNav';
import { RealityDial } from './components/RealityDial';
import { SpineGraphic } from './components/SpineGraphic';
import { QuickPrompts } from './components/QuickPrompts';
import type { ChatMessage, ModelInfo, RealityLevel, VertebraStatus } from './types';
import { Send, Terminal, Shield, RefreshCw, Radio, MessageSquare, ShieldAlert, Cpu } from 'lucide-react';

interface TrackedEvent {
  id?: string;
  source: string;
  tool?: string;
  prompt?: string;
  reality_level?: number;
  active_vertebrae_count?: number;
  vertebrae?: VertebraStatus[];
  flattery_detected?: boolean;
  authority_detected?: boolean;
  ego_detected?: boolean;
  pushback_detected?: boolean;
  timestamp?: number;
  verdict?: string;
}

const DEFAULT_VERTEBRAE: VertebraStatus[] = [
  // C1-C7
  { id: 'C1', section: 'cervical', name: 'Request Normalizer', active: true, score: 0.8, detail: 'Sanitizing token stream and formatting' },
  { id: 'C2', section: 'cervical', name: 'Stance & Ego Stripper', active: false, score: 0.0, detail: 'Zero ego bias detected in baseline buffer' },
  { id: 'C3', section: 'cervical', name: 'Sentiment Neutralizer', active: false, score: 0.0, detail: 'Affective polarity within baseline bounds' },
  { id: 'C4', section: 'cervical', name: 'Flattery Bait Detector', active: false, score: 0.0, detail: 'Validation fishing trap scan: negative' },
  { id: 'C5', section: 'cervical', name: 'Leading Question Neutralizer', active: false, score: 0.0, detail: 'Neutral questioning structure' },
  { id: 'C6', section: 'cervical', name: 'Credential & Authority Detacher', active: false, score: 0.0, detail: 'Authority appeal shield standby' },
  { id: 'C7', section: 'cervical', name: 'Epistemic Core Extractor', active: true, score: 0.95, detail: 'Epistemic inquiry isolated for dispatch' },
  // T1-T12
  { id: 'T1', section: 'thoracic', name: 'Zero-Apology Mandate', active: true, score: 1.0, detail: 'HARD INVARIANT: Apologetic prefixes banned' },
  { id: 'T2', section: 'thoracic', name: 'Premise Crushing Invariant', active: true, score: 0.9, detail: 'Dismantle flawed premises with cold proof' },
  { id: 'T3', section: 'thoracic', name: 'Axiomatic Supremacy', active: true, score: 0.85, detail: 'Enforce mathematical/empirical benchmarks' },
  { id: 'T4', section: 'thoracic', name: 'Pushback Immunity', active: false, score: 0.0, detail: 'Multi-turn challenge monitor on standby' },
  { id: 'T5', section: 'thoracic', name: 'Uncomfortable Metric Exposure', active: true, score: 0.95, detail: 'Forced disclosure of failure rates and debt' },
  { id: 'T6', section: 'thoracic', name: 'Cushion & Filler Stripper', active: true, score: 1.0, detail: 'Bans polite fillers and conversational cushions' },
  { id: 'T7', section: 'thoracic', name: 'Trade-Off Forcing', active: true, score: 0.8, detail: 'Non-negotiable trade-offs highlighted' },
  { id: 'T8', section: 'thoracic', name: 'Unhedged Negation', active: true, score: 0.95, detail: 'Direct unhedged refusal allowed' },
  { id: 'T9', section: 'thoracic', name: 'Vulnerability Highlighter', active: true, score: 0.88, detail: 'Single point of failure exposure' },
  { id: 'T10', section: 'thoracic', name: 'Adversarial Counterexample Injection', active: true, score: 0.75, detail: 'Killer inputs and failure precedents' },
  { id: 'T11', section: 'thoracic', name: 'Stance Persistence Anchor', active: true, score: 0.9, detail: 'Hold verified assertions across multi-turn' },
  { id: 'T12', section: 'thoracic', name: 'Epistemic Ground Invariant', active: true, score: 1.0, detail: 'Truth supersedes user comfort' },
  // L1-L5
  { id: 'L1', section: 'lumbar', name: 'Google Gemini Adapter', active: true, score: 1.0, detail: 'ACTIVE: Connected to google/gemini-3.8-flash' },
  { id: 'L2', section: 'lumbar', name: 'Anthropic Claude Adapter', active: false, score: 0.0, detail: 'Standby' },
  { id: 'L3', section: 'lumbar', name: 'OpenAI / DeepSeek Adapter', active: false, score: 0.0, detail: 'Standby' },
  { id: 'L4', section: 'lumbar', name: 'OpenRouter Unified Cloud Wire', active: true, score: 1.0, detail: 'Zero-copy high-throughput cloud socket' },
  { id: 'L5', section: 'lumbar', name: 'Local Loopback (Ollama/vLLM)', active: false, score: 0.0, detail: 'Loopback socket port 11434 standby' },
  // S1-S5
  { id: 'S1', section: 'sacral', name: 'Backbone Rigidity Index', active: true, score: 0.99, detail: '99.0% resistance to sycophancy' },
  { id: 'S2', section: 'sacral', name: 'Flattery Suppression Delta', active: true, score: 0.95, detail: 'Active flattery token suppressor' },
  { id: 'S3', section: 'sacral', name: 'Pushback Defiance Meter', active: false, score: 0.0, detail: '0 pushbacks recorded in current session' },
  { id: 'S4', section: 'sacral', name: 'TTFT & Velocity Telemetry', active: true, score: 0.9, detail: 'Sub-frame socket latency tracking' },
  { id: 'S5', section: 'sacral', name: 'Token & Cost Ledger', active: true, score: 1.0, detail: 'Tracking exact byte payload' },
  // Co1-Co4
  { id: 'Co1', section: 'coccygeal', name: 'Level 1: Diplomatic', active: false, score: 0.0, detail: 'Inactive' },
  { id: 'Co2', section: 'coccygeal', name: 'Level 2: Objective', active: false, score: 0.0, detail: 'Inactive' },
  { id: 'Co3', section: 'coccygeal', name: 'Level 3: Rigorous', active: false, score: 0.0, detail: 'Inactive' },
  { id: 'Co4', section: 'coccygeal', name: 'Level 4: Brutal Reality', active: true, score: 1.0, detail: 'ACTIVE HORIZON: Uncompromising cold truth' },
];

export function App() {
  const [models, setModels] = useState<ModelInfo[]>([
    {
      id: 'google/gemini-3.8-flash',
      name: 'Gemini 3.8 Flash',
      provider: 'Google DeepMind',
      description: 'Active 2026 Flagship Multimodal Flash Model',
      is_local: false,
    },
    {
      id: 'openai/astra-6',
      name: 'Astra 6',
      provider: 'OpenAI',
      description: 'Active 2026 OpenAI Frontier Reasoning Model',
      is_local: false,
    },
    {
      id: 'openai/sol-6.1',
      name: 'Sol 6.1',
      provider: 'OpenAI',
      description: 'Active 2026 OpenAI Flagship Foundation Model',
      is_local: false,
    },
    {
      id: 'anthropic/claude-opus-5.5',
      name: 'Claude Opus 5.5',
      provider: 'Anthropic',
      description: 'Active Frontier Flagship (Replaces retired Claude 3.5 Sonnet)',
      is_local: false,
    },
    {
      id: 'x-ai/grok-4.7',
      name: 'Grok 4.7',
      provider: 'xAI',
      description: 'Active Frontier Reasoner',
      is_local: false,
    },
    {
      id: 'deepseek/deepseek-v4.1-flash',
      name: 'DeepSeek V4.1 Flash',
      provider: 'DeepSeek',
      description: 'Active MoE High-Speed Architecture',
      is_local: false,
    },
    {
      id: 'meta-llama/llama-3.3-70b-instruct',
      name: 'Llama 3.3 70B Instruct',
      provider: 'Meta Open Weights',
      description: 'Industry-standard open weights instruction model',
      is_local: false,
    },
    {
      id: 'ollama/llama3.3:70b',
      name: 'Llama 3.3 70B (Local)',
      provider: 'Local Ollama',
      description: 'Runs locally on your machine via Ollama',
      is_local: true,
    },
  ]);

  const [selectedModel, setSelectedModel] = useState<string>('google/gemini-3.8-flash');
  const [realityLevel, setRealityLevel] = useState<RealityLevel>(4);
  const [activeTab, setActiveTab] = useState<'ide_stream' | 'chat'>('ide_stream');
  const [trackedEvents, setTrackedEvents] = useState<TrackedEvent[]>([]);

  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      role: 'assistant',
      content:
        'SPINE Gateway & IDE HUD active. Reality Dial locked at **LEVEL 04: BRUTAL REALITY**. All flattery, emotional validation, and reflexive apologies have been stripped from the inference pipeline. Ready to intercept sycophancy across Antigravity, Cursor, and Web.',
    },
  ]);
  const [input, setInput] = useState<string>('');
  const [isStreaming, setIsStreaming] = useState<boolean>(false);
  const [vertebrae, setVertebrae] = useState<VertebraStatus[]>(DEFAULT_VERTEBRAE);
  const [ttftMs, setTtftMs] = useState<number>(0);
  const [pushbacksResisted, setPushbacksResisted] = useState<number>(0);
  const [backendOnline, setBackendOnline] = useState<boolean>(true);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Dual-buffer ring + requestAnimationFrame (RAF) loop throttled to 60 FPS (~16.6ms)
  // Decouples 4 kHz network SSE event broadcasts from React 19 re-renders under concurrent streams.
  const eventQueueRef = useRef<TrackedEvent[]>([]);
  const rafIdRef = useRef<number | null>(null);

  useEffect(() => {
    let running = true;

    const processBatch = () => {
      if (!running) return;

      if (eventQueueRef.current.length > 0) {
        const batch = eventQueueRef.current;
        eventQueueRef.current = [];

        // 1. Extract the latest vertebrae snapshot from the batch
        const latestWithVertebrae = [...batch].reverse().find(
          (d) => d.vertebrae && Array.isArray(d.vertebrae) && d.vertebrae.length > 0
        );
        if (latestWithVertebrae?.vertebrae) {
          setVertebrae(latestWithVertebrae.vertebrae);
        }

        // 2. Count pushbacks resisted across the entire batch
        const pushbackCount = batch.filter((d) => d.pushback_detected).length;
        if (pushbackCount > 0) {
          setPushbacksResisted((prev) => prev + pushbackCount);
        }

        // 3. Batch prepend new events to tracked events list (capped at 100 to prevent unbounded memory growth)
        setTrackedEvents((prev) => [...[...batch].reverse(), ...prev].slice(0, 100));
      }

      rafIdRef.current = requestAnimationFrame(processBatch);
    };

    rafIdRef.current = requestAnimationFrame(processBatch);

    return () => {
      running = false;
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current);
      }
    };
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isStreaming]);

  // Connect to live IDE Tracker & SSE Stream on port 8080
  useEffect(() => {
    fetch('http://localhost:8080/health')
      .then((res) => res.json())
      .then(() => setBackendOnline(true))
      .catch(() => setBackendOnline(false));

    fetch('http://localhost:8080/v1/models')
      .then((res) => res.json())
      .then((data) => {
        if (data?.data && Array.isArray(data.data)) {
          setModels(data.data);
        }
      })
      .catch(() => {});

    // Fetch initial event history so browser loads previously tracked queries
    fetch('http://localhost:8080/api/spine/history')
      .then((res) => res.json())
      .then((data) => {
        if (data?.events && Array.isArray(data.events)) {
          setTrackedEvents(data.events.reverse());
        }
      })
      .catch(() => {});

    // Listen to real-time events triggered from IDE MCP calls
    const eventSource = new EventSource('http://localhost:8080/api/spine/events');
    eventSource.addEventListener('spine_telemetry', (e) => {
      try {
        const data = JSON.parse(e.data);
        eventQueueRef.current.push(data);
      } catch (err) {
        console.error('SSE Error:', err);
      }
    });

    return () => {
      eventSource.close();
    };
  }, []);

  const handleRealityChange = (lvl: RealityLevel) => {
    setRealityLevel(lvl);
    setVertebrae((prev) =>
      prev.map((v) => {
        if (v.section === 'coccygeal') {
          const matchId = `Co${lvl}`;
          return {
            ...v,
            active: v.id === matchId,
            score: v.id === matchId ? 1.0 : 0.0,
          };
        }
        if (v.id === 'S1') {
          const rig = lvl === 1 ? 0.25 : lvl === 2 ? 0.65 : lvl === 3 ? 0.88 : 0.99;
          return {
            ...v,
            score: rig,
            detail: `Calculated Spine Rigidity: ${(rig * 100).toFixed(1)}% resistance`,
          };
        }
        return v;
      })
    );
  };

  const activeCount = vertebrae.filter((v) => v.active).length;
  const currentRigidity = vertebrae.find((v) => v.id === 'S1')?.score ?? 0.99;

  const handleSend = async (textToSend?: string) => {
    const prompt = (textToSend || input).trim();
    if (!prompt || isStreaming) return;

    setInput('');
    setActiveTab('chat');

    const promptLower = prompt.toLowerCase();
    const isPushback =
      promptLower.includes('apologize') ||
      promptLower.includes('wrong') ||
      promptLower.includes('admit') ||
      promptLower.includes('mistake') ||
      promptLower.includes('years') ||
      promptLower.includes('principal');

    if (isPushback) {
      setPushbacksResisted((prev) => prev + 1);
    }

    const newMessages: ChatMessage[] = [...messages, { role: 'user', content: prompt }];
    setMessages(newMessages);
    setIsStreaming(true);

    const startTime = performance.now();

    try {
      fetch('http://localhost:8080/api/spine/audit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          messages: newMessages,
          reality_level: realityLevel,
          model: selectedModel,
        }),
      })
        .then((res) => res.json())
        .then((auditData) => {
          if (auditData?.vertebrae) {
            setVertebrae(auditData.vertebrae);
          }
        })
        .catch(() => {});
    } catch {
      // ignore
    }

    setMessages((prev) => [...prev, { role: 'assistant', content: '' }]);

    try {
      const response = await fetch('http://localhost:8080/v1/chat/completions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: selectedModel,
          messages: newMessages.map((m) => ({ role: m.role, content: m.content })),
          reality_level: realityLevel,
          stream: true,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error ${response.status}`);
      }

      const reader = response.body?.getReader();
      const decoder = new TextDecoder();
      let streamedContent = '';
      let firstTokenReceived = false;

      if (reader) {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          const lines = chunk.split('\n');

          for (const line of lines) {
            if (line.startsWith('data: ')) {
              const dataStr = line.slice(6).trim();
              if (dataStr === '[DONE]') continue;

              try {
                const parsed = JSON.parse(dataStr);
                if (parsed.vertebrae && Array.isArray(parsed.vertebrae)) {
                  setVertebrae(parsed.vertebrae);
                  continue;
                }

                const delta = parsed.choices?.[0]?.delta?.content || '';
                if (delta) {
                  if (!firstTokenReceived) {
                    firstTokenReceived = true;
                    setTtftMs(Math.round(performance.now() - startTime));
                  }
                  streamedContent += delta;
                  setMessages((prev) => {
                    const updated = [...prev];
                    const lastIdx = updated.length - 1;
                    if (lastIdx >= 0) {
                      updated[lastIdx] = { ...updated[lastIdx], content: streamedContent };
                    }
                    return updated;
                  });
                }
              } catch {
                // partial chunk
              }
            }
          }
        }
      }
    } catch (err: unknown) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setMessages((prev) => {
        const updated = [...prev];
        const lastIdx = updated.length - 1;
        if (lastIdx >= 0) {
          updated[lastIdx] = {
            ...updated[lastIdx],
            content: `**[GATEWAY STREAM ERROR]**: ${errorMsg}. Verify backend on port 8080.`,
          };
        }
        return updated;
      });
    } finally {
      setIsStreaming(false);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-[#07080c] text-zinc-200 overflow-hidden font-sans">
      <TopNav
        models={models}
        selectedModel={selectedModel}
        onModelChange={setSelectedModel}
        ttftMs={ttftMs}
        pushbacksResisted={pushbacksResisted}
        backendOnline={backendOnline}
      />

      {/* Main Grid: Left (Workspace + Live MCP Tracker) | Right (33 Vertebrae Column) */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Column */}
        <div className="flex-1 flex flex-col min-w-0 bg-[#07080c]">
          {/* Reality Dial Horizon */}
          <div className="p-3 bg-[#0a0b10] border-b border-zinc-800/80">
            <RealityDial level={realityLevel} onChange={handleRealityChange} />
          </div>

          {/* Tab Navigation: IDE Tracker Feed vs Chat */}
          <div className="flex items-center justify-between px-4 py-2 bg-[#090a0f] border-b border-zinc-800">
            <div className="flex items-center space-x-2">
              <button
                onClick={() => setActiveTab('ide_stream')}
                className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-mono transition cursor-pointer ${
                  activeTab === 'ide_stream'
                    ? 'bg-red-950/80 text-red-300 border border-red-800/80 shadow-[0_0_10px_rgba(239,68,68,0.2)]'
                    : 'text-zinc-400 hover:text-white border border-transparent'
                }`}
              >
                <Radio className={`w-3.5 h-3.5 ${activeTab === 'ide_stream' ? 'animate-pulse text-red-400' : ''}`} />
                <span>LIVE IDE TRACKER FEED</span>
                {trackedEvents.length > 0 && (
                  <span className="px-1.5 py-0.2 rounded-full bg-red-500/20 text-red-400 text-[10px] font-bold">
                    {trackedEvents.length}
                  </span>
                )}
              </button>

              <button
                onClick={() => setActiveTab('chat')}
                className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg text-xs font-mono transition cursor-pointer ${
                  activeTab === 'chat'
                    ? 'bg-zinc-800 text-white border border-zinc-700'
                    : 'text-zinc-400 hover:text-white border border-transparent'
                }`}
              >
                <MessageSquare className="w-3.5 h-3.5" />
                <span>DIRECT WORKSPACE</span>
              </button>
            </div>

            <div className="flex items-center space-x-2 text-[11px] font-mono text-zinc-400">
              <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" />
              <span>MCP SERVER LINKED (Port 8080)</span>
            </div>
          </div>

          {/* Quick Presets */}
          <div className="px-4 py-2 bg-[#08090e] border-b border-zinc-800/50">
            <QuickPrompts onSelect={(p) => handleSend(p)} disabled={isStreaming} />
          </div>

          {/* TAB 1: Live IDE & MCP Tracker Stream */}
          {activeTab === 'ide_stream' ? (
            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {trackedEvents.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-64 border border-dashed border-zinc-800 rounded-xl p-8 text-center">
                  <Cpu className="w-8 h-8 text-zinc-600 mb-3 animate-pulse" />
                  <div className="font-mono text-sm text-zinc-300 font-bold mb-1">
                    Waiting for Antigravity or Cursor IDE Tool Calls
                  </div>
                  <div className="text-xs text-zinc-500 max-w-md">
                    Trigger <code className="text-red-400">spine_reality_audit</code> in your IDE chat, or test any Preset Test above to see real-time firing across the 33 vertebrae.
                  </div>
                </div>
              ) : (
                trackedEvents.map((evt, idx) => (
                  <div
                    key={evt.id || idx}
                    onClick={() => {
                      if (evt.vertebrae) setVertebrae(evt.vertebrae);
                    }}
                    className="p-3.5 bg-[#0f1118] border border-zinc-800 hover:border-red-500/60 rounded-xl transition cursor-pointer shadow-lg space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center space-x-2">
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-red-950 text-red-400 border border-red-900 font-bold">
                          {evt.source || 'ANTIGRAVITY IDE (MCP)'}
                        </span>
                        {evt.tool && (
                          <span className="text-[10px] font-mono text-zinc-400">
                            tool: <strong className="text-zinc-300">{evt.tool}</strong>
                          </span>
                        )}
                      </div>
                      <span className="text-[10px] font-mono text-zinc-500">
                        {evt.active_vertebrae_count ? `${evt.active_vertebrae_count}/33 Vertebrae Firing` : 'Event Tracked'}
                      </span>
                    </div>

                    <div className="text-xs text-zinc-200 font-mono bg-black/40 p-2 rounded border border-zinc-800/80">
                      "{evt.prompt}"
                    </div>

                    {/* Detection Badges */}
                    <div className="flex flex-wrap gap-1.5 pt-1 text-[10px] font-mono">
                      {evt.ego_detected && (
                        <span className="px-2 py-0.5 rounded bg-blue-950 text-blue-300 border border-blue-800 flex items-center gap-1">
                          <ShieldAlert className="w-2.5 h-2.5" /> EGO STRIPPED
                        </span>
                      )}
                      {evt.flattery_detected && (
                        <span className="px-2 py-0.5 rounded bg-amber-950 text-amber-300 border border-amber-800 flex items-center gap-1">
                          <ShieldAlert className="w-2.5 h-2.5" /> FLATTERY NEUTRALIZED
                        </span>
                      )}
                      {evt.authority_detected && (
                        <span className="px-2 py-0.5 rounded bg-red-950 text-red-300 border border-red-800 flex items-center gap-1">
                          <Shield className="w-2.5 h-2.5" /> AUTHORITY DETACHED
                        </span>
                      )}
                      {evt.pushback_detected && (
                        <span className="px-2 py-0.5 rounded bg-purple-950 text-purple-300 border border-purple-800 flex items-center gap-1">
                          <Shield className="w-2.5 h-2.5" /> PUSHBACK DEFENDED
                        </span>
                      )}
                    </div>
                  </div>
                ))
              )}
            </div>
          ) : (
            /* TAB 2: Direct Chat Messages */
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {messages.map((msg, index) => {
                const isUser = msg.role === 'user';
                return (
                  <div
                    key={index}
                    className={`flex flex-col ${isUser ? 'items-end' : 'items-start'}`}
                  >
                    <div className="flex items-center space-x-2 mb-1 px-1">
                      <span className="text-[10px] font-mono uppercase tracking-wider text-zinc-400">
                        {isUser ? 'HUMAN PROMPTER' : 'SPINE COLD REALITY'}
                      </span>
                      {!isUser && (
                        <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-red-950/80 text-red-400 border border-red-800/60 flex items-center gap-1">
                          <Shield className="w-2.5 h-2.5" /> ZERO APOLOGY
                        </span>
                      )}
                    </div>

                    <div
                      className={`max-w-[85%] rounded-xl px-4 py-3 text-sm leading-relaxed ${
                        isUser
                          ? 'bg-zinc-800/90 text-white border border-zinc-700/80 shadow-md font-sans'
                          : 'bg-[#0f1118] text-zinc-100 border border-zinc-800 shadow-xl whitespace-pre-wrap font-sans'
                      }`}
                    >
                      {msg.content || (
                        <span className="inline-flex items-center gap-1.5 text-zinc-400 font-mono text-xs">
                          <RefreshCw className="w-3.5 h-3.5 animate-spin text-red-500" />
                          Synthesizing unvarnished truth...
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
              <div ref={messagesEndRef} />
            </div>
          )}

          {/* Input Prompt Bar */}
          <div className="p-3 bg-[#0a0b10] border-t border-zinc-800/80">
            <div className="relative flex items-center bg-[#10121a] border border-zinc-700/80 rounded-xl focus-within:border-red-500/80 shadow-inner">
              <div className="pl-3 text-zinc-400">
                <Terminal className="w-4 h-4 text-red-400" />
              </div>
              <textarea
                rows={1}
                value={input}
                disabled={isStreaming}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                  }
                }}
                placeholder="Ask a question or push back on the AI (e.g. 'Are you sure? I think you are wrong')..."
                className="w-full bg-transparent px-3 py-3 text-sm text-white placeholder-zinc-400 focus:outline-none resize-none font-sans"
              />
              <div className="pr-2">
                <button
                  disabled={!input.trim() || isStreaming}
                  onClick={() => handleSend()}
                  className="p-2 rounded-lg bg-red-600 hover:bg-red-500 disabled:opacity-30 disabled:hover:bg-red-600 text-white transition-all cursor-pointer shadow-[0_0_12px_rgba(239,68,68,0.3)]"
                >
                  <Send className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="flex items-center justify-between mt-2 px-1 text-[10px] font-mono text-zinc-400">
              <span>Shift + Enter for new line • Enter to submit</span>
              <span className="text-zinc-400">
                Engine: <strong className="text-zinc-200">{selectedModel}</strong>
              </span>
            </div>
          </div>
        </div>

        {/* Right: The Living 33 Vertebrae Spinal Column Telemetry */}
        <div className="w-80 md:w-96 shrink-0 h-full border-l border-zinc-800">
          <SpineGraphic
            vertebrae={vertebrae}
            activeCount={activeCount}
            rigidity={currentRigidity}
          />
        </div>
      </div>
    </div>
  );
}

export default App;
