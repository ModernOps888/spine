import React, { useState } from 'react';
import type { VertebraStatus } from '../types';
import { Activity, ShieldCheck, Info } from 'lucide-react';

interface SpineGraphicProps {
  vertebrae: VertebraStatus[];
  activeCount: number;
  rigidity: number;
}

const sectionConfig = {
  cervical: {
    label: 'Cervical (C1–C7)',
    desc: 'Cognitive & Ego Stripping',
    color: 'border-cyan-500/60 bg-cyan-950/40 text-cyan-300 shadow-[0_0_12px_rgba(6,182,212,0.25)]',
    dot: 'bg-cyan-400',
    accentText: 'text-cyan-400',
  },
  thoracic: {
    label: 'Thoracic (T1–T12)',
    desc: 'The 12 Invariant Rules',
    color: 'border-amber-500/60 bg-amber-950/40 text-amber-300 shadow-[0_0_12px_rgba(245,158,11,0.25)]',
    dot: 'bg-amber-400',
    accentText: 'text-amber-400',
  },
  lumbar: {
    label: 'Lumbar (L1–L5)',
    desc: 'Weight-Bearing Adapters',
    color: 'border-emerald-500/60 bg-emerald-950/40 text-emerald-300 shadow-[0_0_12px_rgba(16,185,129,0.25)]',
    dot: 'bg-emerald-400',
    accentText: 'text-emerald-400',
  },
  sacral: {
    label: 'Sacral (S1–S5)',
    desc: 'Spine Telemetry Core',
    color: 'border-red-500/60 bg-red-950/40 text-red-300 shadow-[0_0_12px_rgba(239,68,68,0.25)]',
    dot: 'bg-red-400',
    accentText: 'text-red-400',
  },
  coccygeal: {
    label: 'Coccygeal (Co1–Co4)',
    desc: 'Reality Dial Anchors',
    color: 'border-purple-500/60 bg-purple-950/40 text-purple-300 shadow-[0_0_12px_rgba(168,85,247,0.25)]',
    dot: 'bg-purple-400',
    accentText: 'text-purple-400',
  },
};

export const SpineGraphic: React.FC<SpineGraphicProps> = ({
  vertebrae,
  activeCount,
  rigidity,
}) => {
  const [selectedVertebra, setSelectedVertebra] = useState<VertebraStatus | null>(null);

  // Group vertebrae by anatomical section
  const sections: { key: keyof typeof sectionConfig; items: VertebraStatus[] }[] = [
    { key: 'cervical', items: vertebrae.filter((v) => v.section === 'cervical') },
    { key: 'thoracic', items: vertebrae.filter((v) => v.section === 'thoracic') },
    { key: 'lumbar', items: vertebrae.filter((v) => v.section === 'lumbar') },
    { key: 'sacral', items: vertebrae.filter((v) => v.section === 'sacral') },
    { key: 'coccygeal', items: vertebrae.filter((v) => v.section === 'coccygeal') },
  ];

  return (
    <div className="flex flex-col h-full bg-[#0d0f15] border-l border-zinc-800/80 p-4 select-none">
      {/* Telemetry Header */}
      <div className="mb-4 pb-3 border-b border-zinc-800">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Activity className="w-4 h-4 text-red-500 animate-pulse" />
            <span className="text-xs font-mono font-bold tracking-wider text-zinc-300 uppercase">
              33 Vertebrae Core
            </span>
          </div>
          <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
            {activeCount}/33 Firing
          </span>
        </div>

        <div className="mt-3 flex items-center justify-between text-xs">
          <span className="text-zinc-500 font-mono">Backbone Rigidity</span>
          <span className="font-mono font-bold text-red-400">{(rigidity * 100).toFixed(1)}%</span>
        </div>
        <div className="w-full bg-zinc-900 rounded-full h-1.5 mt-1 overflow-hidden border border-zinc-800">
          <div
            className="bg-gradient-to-r from-amber-500 to-red-500 h-full transition-all duration-500 rounded-full"
            style={{ width: `${rigidity * 100}%` }}
          />
        </div>
      </div>

      {/* Selected Vertebra Inspector Card */}
      {selectedVertebra ? (
        <div className="mb-4 p-3 bg-zinc-900/90 border border-zinc-700/80 rounded-lg text-xs font-mono animate-fadeIn">
          <div className="flex items-center justify-between text-zinc-400 mb-1">
            <span className="font-bold text-white flex items-center gap-1.5">
              <span className={`w-2 h-2 rounded-full ${selectedVertebra.active ? 'bg-emerald-400 animate-ping' : 'bg-zinc-600'}`} />
              [{selectedVertebra.id}] {selectedVertebra.name}
            </span>
            <span className={selectedVertebra.active ? 'text-emerald-400 font-semibold' : 'text-zinc-500'}>
              {selectedVertebra.active ? 'TRIGGERED' : 'INACTIVE'}
            </span>
          </div>
          <div className="text-[11px] text-zinc-300 mt-2 bg-black/40 p-2 rounded border border-zinc-800">
            {selectedVertebra.detail}
          </div>
          <div className="mt-2 text-[10px] text-zinc-500 flex justify-between">
            <span>Score: {(selectedVertebra.score * 100).toFixed(0)}%</span>
            <span>Section: {selectedVertebra.section.toUpperCase()}</span>
          </div>
        </div>
      ) : (
        <div className="mb-4 p-2.5 bg-zinc-900/40 border border-zinc-800/80 rounded-lg text-[11px] text-zinc-500 flex items-center gap-2">
          <Info className="w-3.5 h-3.5 text-zinc-400 shrink-0" />
          <span>Click any vertebra to inspect its live epistemic diagnostic.</span>
        </div>
      )}

      {/* Vertical Spinal Column */}
      <div className="flex-1 overflow-y-auto pr-1 space-y-4">
        {sections.map((section) => {
          const cfg = sectionConfig[section.key];
          return (
            <div key={section.key} className="space-y-1.5">
              <div className="flex items-center justify-between text-[10px] font-mono text-zinc-500 uppercase tracking-wider px-1">
                <span className={cfg.accentText}>{cfg.label}</span>
                <span className="text-zinc-600 text-[9px]">{cfg.desc}</span>
              </div>

              <div className="relative pl-3 space-y-1.5 border-l border-zinc-800 ml-2">
                {section.items.map((vert) => {
                  const isActive = vert.active;
                  const isSelected = selectedVertebra?.id === vert.id;
                  return (
                    <button
                      key={vert.id}
                      onClick={() => setSelectedVertebra(vert)}
                      className={`w-full text-left flex items-center justify-between px-2.5 py-1.5 rounded-md border text-xs font-mono transition-all duration-200 ${
                        isSelected
                          ? 'border-white bg-zinc-800 text-white shadow-lg'
                          : isActive
                          ? cfg.color
                          : 'border-zinc-800/60 bg-zinc-900/30 text-zinc-500 hover:border-zinc-700 hover:text-zinc-400'
                      }`}
                    >
                      <div className="flex items-center space-x-2 truncate">
                        <span
                          className={`w-1.5 h-1.5 rounded-full ${
                            isActive ? cfg.dot : 'bg-zinc-700'
                          }`}
                        />
                        <span className="font-bold shrink-0">{vert.id}</span>
                        <span className="truncate text-[11px] font-sans">{vert.name}</span>
                      </div>

                      {isActive && (
                        <ShieldCheck className="w-3.5 h-3.5 shrink-0 ml-1 text-emerald-400" />
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
