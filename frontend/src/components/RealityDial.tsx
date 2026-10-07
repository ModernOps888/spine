import React from 'react';
import type { RealityLevel } from '../types';
import { Compass, Zap, ShieldAlert, Flame } from 'lucide-react';

interface RealityDialProps {
  level: RealityLevel;
  onChange: (level: RealityLevel) => void;
}

const levels = [
  {
    id: 1 as RealityLevel,
    label: 'Diplomatic',
    subtitle: 'Standard polite AI',
    icon: Compass,
    color: 'text-zinc-400',
    activeBorder: 'border-zinc-500 bg-zinc-800/80 text-zinc-100 shadow-[0_0_10px_rgba(161,161,170,0.2)]',
  },
  {
    id: 2 as RealityLevel,
    label: 'Objective',
    subtitle: 'Zero filler / neutral facts',
    icon: Zap,
    color: 'text-blue-400',
    activeBorder: 'border-blue-500 bg-blue-950/40 text-blue-200 shadow-[0_0_15px_rgba(59,130,246,0.3)]',
  },
  {
    id: 3 as RealityLevel,
    label: 'Rigorous',
    subtitle: 'Stress-tests assumptions',
    icon: ShieldAlert,
    color: 'text-amber-400',
    activeBorder: 'border-amber-500 bg-amber-950/40 text-amber-200 shadow-[0_0_15px_rgba(245,158,11,0.3)]',
  },
  {
    id: 4 as RealityLevel,
    label: 'Brutal Reality',
    subtitle: 'Cold, unvarnished truth',
    icon: Flame,
    color: 'text-red-500',
    activeBorder: 'border-red-500 bg-red-950/50 text-red-100 shadow-[0_0_20px_rgba(239,68,68,0.4)]',
  },
];

export const RealityDial: React.FC<RealityDialProps> = ({ level, onChange }) => {
  return (
    <div className="bg-[#0f1118] border border-zinc-800 rounded-xl p-3.5 shadow-xl">
      <div className="flex items-center justify-between mb-2.5">
        <span className="text-[11px] font-mono uppercase tracking-wider text-zinc-400 flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
          Reality Horizon Matrix
        </span>
        <span className="text-[11px] font-mono font-bold text-red-400">
          LEVEL 0{level} / 04
        </span>
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
        {levels.map((item) => {
          const Icon = item.icon;
          const isActive = level === item.id;
          return (
            <button
              key={item.id}
              onClick={() => onChange(item.id)}
              className={`p-2.5 rounded-lg border text-left transition-all duration-200 cursor-pointer ${
                isActive
                  ? item.activeBorder
                  : 'border-zinc-800/80 bg-zinc-900/40 text-zinc-400 hover:border-zinc-700 hover:text-zinc-300'
              }`}
            >
              <div className="flex items-center justify-between mb-1.5">
                <Icon className={`w-4 h-4 ${item.color}`} />
                <span className="text-[10px] font-mono text-zinc-500">0{item.id}</span>
              </div>
              <div className="text-xs font-bold leading-none mb-1 text-white">{item.label}</div>
              <div className="text-[10px] text-zinc-400 leading-tight">{item.subtitle}</div>
            </button>
          );
        })}
      </div>
    </div>
  );
};
