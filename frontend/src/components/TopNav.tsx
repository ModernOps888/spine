import React from 'react';
import { Shield, Cpu, Clock, Flame } from 'lucide-react';
import type { ModelInfo } from '../types';

interface TopNavProps {
  models: ModelInfo[];
  selectedModel: string;
  onModelChange: (model: string) => void;
  ttftMs: number;
  pushbacksResisted: number;
  backendOnline: boolean;
}

export const TopNav: React.FC<TopNavProps> = ({
  models,
  selectedModel,
  onModelChange,
  ttftMs,
  pushbacksResisted,
  backendOnline,
}) => {
  return (
    <header className="h-14 bg-[#0a0b10] border-b border-zinc-800 px-4 flex items-center justify-between select-none">
      {/* Brand Identity */}
      <div className="flex items-center space-x-3">
        <div className="w-8 h-8 rounded-lg bg-red-950/60 border border-red-500/60 flex items-center justify-center text-red-400 shadow-[0_0_15px_rgba(239,68,68,0.3)]">
          <Flame className="w-5 h-5" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <span className="font-mono font-black text-sm tracking-wider text-white">SPINE</span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-red-950/80 text-red-400 border border-red-800/80">
              GATEWAY
            </span>
          </div>
          <span className="text-[10px] text-zinc-400 tracking-tight block">
            Anti-Sycophancy Reality Engine • 33 Vertebrae Core
          </span>
        </div>
      </div>

      {/* Model Selector & Live Telemetry Badges */}
      <div className="flex items-center space-x-3">
        {/* Real TTFT Timer */}
        <div className="hidden md:flex items-center space-x-1.5 px-2.5 py-1 rounded bg-zinc-900 border border-zinc-800 text-[11px] font-mono text-zinc-300">
          <Clock className="w-3.5 h-3.5 text-cyan-400" />
          <span>TTFT:</span>
          <span className="text-white font-bold">{ttftMs > 0 ? `${ttftMs}ms` : '--'}</span>
        </div>

        {/* Real Pushbacks Resisted Counter */}
        <div className="hidden sm:flex items-center space-x-1.5 px-2.5 py-1 rounded bg-zinc-900 border border-zinc-800 text-[11px] font-mono text-zinc-300">
          <Shield className="w-3.5 h-3.5 text-amber-400" />
          <span>Spine Defiance:</span>
          <span className="text-amber-400 font-bold">{pushbacksResisted}</span>
        </div>

        {/* Model Picker */}
        <div className="flex items-center space-x-1.5 bg-zinc-900 border border-zinc-700/80 rounded-lg px-2 py-1 text-xs">
          <Cpu className="w-3.5 h-3.5 text-zinc-400" />
          <select
            value={selectedModel}
            onChange={(e) => onModelChange(e.target.value)}
            className="bg-transparent text-white font-mono text-xs focus:outline-none cursor-pointer"
          >
            {models.map((m) => (
              <option key={m.id} value={m.id} className="bg-zinc-900 text-white">
                {m.name} {m.is_local ? '(Local)' : ''}
              </option>
            ))}
          </select>
        </div>

        {/* Live Status indicator */}
        <div className="flex items-center space-x-1.5 text-[11px] font-mono pl-1">
          <span
            className={`w-2 h-2 rounded-full ${
              backendOnline ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]' : 'bg-red-500'
            }`}
          />
          <span className="hidden lg:inline text-zinc-400">
            {backendOnline ? 'SYSTEM NOMINAL' : 'OFFLINE'}
          </span>
        </div>
      </div>
    </header>
  );
};
