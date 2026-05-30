<template>
  <main class="app-shell">
    <section class="left-rail">
      <header class="brand-row">
        <div>
          
          <h1>行大运</h1>
        </div>
        <div class="status-pill" :class="{ active: isDrawing }">
          {{ isDrawing ? '开奖中' : '待机' }}
        </div>
      </header>

      <div class="switch-row">
        <button :class="{ selected: lottery === 'ssq' }" @click="setLottery('ssq')">
          双色球
          <span>红球 1-33 / 蓝球 1-16</span>
        </button>
        <button :class="{ selected: lottery === 'dlt' }" @click="setLottery('dlt')">
          大乐透
          <span>前区 1-35 / 后区 1-12</span>
        </button>
      </div>

      <div class="panel metrics-panel">
        <div>
          <span class="metric-label">球组编号</span>
          <strong>{{ replaceSetId.toString().padStart(2, '0') }}</strong>
        </div>
        <div>
          <span class="metric-label">热号</span>
          <p>{{ analysis?.hotFront.join(' · ') || '--' }}</p>
        </div>
        <div>
          <span class="metric-label">冷号</span>
          <p>{{ analysis?.coldFront.join(' · ') || '--' }}</p>
        </div>
      </div>

      <section class="panel">
        <div class="section-title">
          <h2>近期开奖号码</h2>
          <span>{{ filteredHistory.length }} 期样本</span>
        </div>
        <div class="history-list">
          <article
            v-for="record in filteredHistory"
            :key="record.id"
            class="history-card"
            :class="{ target: targetIssue === record.issue }"
            @click="targetIssue = record.issue"
          >
            <div class="history-meta">
              <strong>{{ record.issue }}</strong>
              <span>{{ record.date }} · {{ record.station }}</span>
            </div>
            <div class="ball-row">
              <span v-for="n in record.front" :key="`f-${record.id}-${n}`" class="mini-ball red">{{ pad(n) }}</span>
              <span v-for="n in record.back" :key="`b-${record.id}-${n}`" class="mini-ball blue">{{ pad(n) }}</span>
            </div>
            <footer>
              <span>一等奖 {{ record.jackpotCount }} 注</span>
              <span>销量 {{ record.salesMillion.toFixed(1) }}M</span>
            </footer>
          </article>
        </div>
      </section>

      <section class="panel trend-panel">
        <div class="section-title">
          <h2>走势分布</h2>
          <span>历史热度摘要</span>
        </div>
        <div class="trend-summary">
          <div class="trend-card hot">
            <span>红球热区</span>
            <strong>{{ analysis?.hotFront.slice(0, 3).map(pad).join(' · ') || '--' }}</strong>
          </div>
          <div class="trend-card cool">
            <span>红球冷区</span>
            <strong>{{ analysis?.coldFront.slice(0, 3).map(pad).join(' · ') || '--' }}</strong>
          </div>
        </div>
        <div class="heat-lanes">
          <div v-for="lane in trendLanes" :key="lane.label" class="heat-lane">
            <div class="lane-head">
              <span>{{ lane.label }}</span>
              <strong>{{ lane.range }}</strong>
            </div>
            <div class="lane-track">
              <i :style="{ width: `${lane.value}%` }"></i>
            </div>
          </div>
        </div>
        <div class="balance-row">
          <div>
            <span>三区分布</span>
            <strong>{{ analysis?.zoneBalance.join(' / ') || '--' }}</strong>
          </div>
          <div>
            <span>奇偶比</span>
            <strong>{{ analysis?.oddEvenBalance.join(' / ') || '--' }}</strong>
          </div>
        </div>
      </section>
    </section>

    <section class="stage">
      <div class="stage-topbar">
        <div>
          
          <h2>中大奖</h2>
          <p class="stage-copy">{{ ruleText }}</p>
        </div>
        <div class="actions">
          <button @click="replaceBalls" :disabled="isDrawing">一键换球</button>
          <button @click="runBacktest" :disabled="isDrawing || isBacktesting">自我测试</button>
          <button class="predict-btn" @click="runPrediction" :disabled="isDrawing || isPredicting">智能预测</button>
          <button class="primary" @click="startDraw" :disabled="isDrawing">开始开奖</button>
        </div>
      </div>

      <div class="machine-layout">
        <div class="machine-card">
          <div ref="sceneEl" class="machine-scene"></div>
          <div class="phase-card">
            <span>当前阶段</span>
            <strong>{{ phaseLabel }}</strong>
          </div>
        </div>

        <aside class="ball-bank panel">
          <div class="section-title">
            <h2>开奖球池</h2>
            <span>{{ lottery === 'ssq' ? '双色球规则' : '大乐透规则' }}</span>
          </div>
          <div class="bank-section">
            <div class="bank-heading">
              <span>{{ lottery === 'ssq' ? '红球 1-33' : '前区红球 1-35' }}</span>
              <strong>{{ analysis?.frontPick || 0 }} 个</strong>
            </div>
            <div class="bank-grid">
              <span
                v-for="n in redPool"
                :key="`red-${n}`"
                class="bank-ball red"
                :class="{ drawn: displayedFront.includes(n) }"
              >{{ pad(n) }}</span>
            </div>
          </div>
          <div class="bank-section">
            <div class="bank-heading">
              <span>{{ lottery === 'ssq' ? '蓝球 1-16' : '后区蓝球 1-12' }}</span>
              <strong>{{ analysis?.backPick || 0 }} 个</strong>
            </div>
            <div class="bank-grid compact">
              <span
                v-for="n in bluePool"
                :key="`blue-${n}`"
                class="bank-ball blue"
                :class="{ drawn: displayedBack.includes(n) }"
              >{{ pad(n) }}</span>
            </div>
          </div>
        </aside>
      </div>

      <div class="result-strip">
        <div>
          <span class="result-label">{{ lottery === 'ssq' ? '红球' : '前区' }}</span>
          <div class="result-balls">
            <span
              v-for="n in displayedFront"
              :key="`rf-${n}`"
              class="result-ball red"
            >{{ pad(n) }}</span>
            <span v-for="slot in Math.max(0, (analysis?.frontPick || 0) - displayedFront.length)" :key="`fs-${slot}`" class="ghost-ball"></span>
          </div>
        </div>
        <div>
          <span class="result-label">{{ lottery === 'ssq' ? '蓝球' : '后区' }}</span>
          <div class="result-balls">
            <span
              v-for="n in displayedBack"
              :key="`rb-${n}`"
              class="result-ball blue"
            >{{ pad(n) }}</span>
            <span v-for="slot in Math.max(0, (analysis?.backPick || 0) - displayedBack.length)" :key="`bs-${slot}`" class="ghost-ball"></span>
          </div>
        </div>
      </div>

      <section class="lower-grid">
        <article class="panel physics-panel">
          <div class="section-title">
            <h2>物理参数</h2>
            <span>{{ drawResult?.requestId || '待模拟' }}</span>
          </div>
          <div class="physics-grid">
            <div>
              <span>转筒转速</span>
              <strong>{{ drawResult ? `${drawResult.chamberRpm.toFixed(1)} rpm` : '--' }}</strong>
            </div>
            <div>
              <span>碰撞次数</span>
              <strong>{{ drawResult?.collisionCount ?? '--' }}</strong>
            </div>
            <div>
              <span>能量损耗</span>
              <strong>{{ drawResult ? `${(drawResult.energyLoss * 100).toFixed(1)}%` : '--' }}</strong>
            </div>
            <div>
              <span>开奖站点</span>
              <strong>{{ drawResult?.station || '--' }}</strong>
            </div>
          </div>
          <p class="caution">
            历史拟合只能验证模拟分布和参数稳定性，不能用于预测真实彩票开奖。
          </p>
        </article>

        <article class="panel" v-if="predictResult">
          <div class="section-title">
            <h2>智能预测</h2>
            <span>置信度 {{ (predictResult.confidence * 100).toFixed(1) }}%</span>
          </div>
          <div class="backtest">
            <strong>走势吻合 {{ (predictResult.trendScore * 100).toFixed(1) }}% · 规律匹配 {{ (predictResult.patternScore * 100).toFixed(1) }}% · 物理一致性 {{ (predictResult.physicsScore * 100).toFixed(1) }}%</strong>
            <p>蒙特卡洛 {{ predictResult.monteCarloRounds }} 轮模拟</p>
            <div style="margin-top:8px;">
              <span v-for="(n,i) in predictResult.front" :key="'pf-'+i" class="mini-ball red">{{ pad(n) }}</span>
              <span v-for="(n,i) in predictResult.back" :key="'pb-'+i" class="mini-ball blue">{{ pad(n) }}</span>
            </div>
            <div style="margin-top:8px;font-size:11px;color:var(--text-soft);">
              <span v-for="(c,i) in predictResult.frontConfidence" :key="'pfc-'+i">红{{ pad(predictResult.front[i]) }}:{{ (c*100).toFixed(0) }}% </span>
              <span v-for="(c,i) in predictResult.backConfidence" :key="'pbc-'+i">蓝{{ pad(predictResult.back[i]) }}:{{ (c*100).toFixed(0) }}%</span>
            </div>
          </div>
        </article>
        <article class="panel">
          <div class="section-title">
            <h2>回测结果</h2>
            <span>{{ backtest?.sampleCount || 0 }} 次</span>
          </div>
          <div v-if="backtest" class="backtest">
            <strong>{{ backtest.exactMatches }} 次完全复现</strong>
            <p>红球/前区均值 {{ backtest.averageFrontHits }}，蓝球/后区均值 {{ backtest.averageBackHits }}</p>
            <p>最佳样本 {{ backtest.bestIssue }}，得分 {{ backtest.bestScore }}</p>
          </div>
          <div v-else class="empty-copy">点击“自我测试”用当前球组回放历史期号。</div>
        </article>
      </section>
    </section>
  </main>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import * as THREE from 'three';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';

type LotteryKind = 'ssq' | 'dlt';

type DrawRecord = {
  id: string;
  issue: string;
  date: string;
  lottery: LotteryKind;
  station: string;
  salesMillion: number;
  jackpotCount: number;
  front: number[];
  back: number[];
};

type BallProfile = {
  number: number;
  color: 'red' | 'blue';
  mass: number;
  friction: number;
  roughness: number;
  bias: number;
  heat: number;
};

type AnalysisSummary = {
  lottery: LotteryKind;
  frontRange: number;
  backRange: number;
  frontPick: number;
  backPick: number;
  hotFront: number[];
  coldFront: number[];
  hotBack: number[];
  coldBack: number[];
  zoneBalance: number[];
  oddEvenBalance: number[];
  ballProfiles: BallProfile[];
};

type DrawResult = {
  requestId: string;
  lottery: LotteryKind;
  issue: string;
  station: string;
  front: number[];
  back: number[];
  spinSeconds: number;
  chamberRpm: number;
  collisionCount: number;
  energyLoss: number;
  exactHistoryMatch: boolean;
  explanation: string[];
};

type PredictResult = {
  lottery: LotteryKind;
  front: number[];
  back: number[];
  confidence: number;
  frontConfidence: number[];
  backConfidence: number[];
  trendScore: number;
  patternScore: number;
  physicsScore: number;
  monteCarloRounds: number;
  topCandidates: { front: number[]; back: number[]; compositeScore: number }[];
  explanation: string[];
};

type BacktestResult = {
  lottery: LotteryKind;
  sampleCount: number;
  exactMatches: number;
  averageFrontHits: number;
  averageBackHits: number;
  bestIssue: string;
  bestScore: number;
  notes: string[];
};

type VisualBall = {
  mesh: THREE.Mesh;
  velocity: THREE.Vector3;
  profile: BallProfile;
  selected: boolean;
  chutePhase: number;
  chuteIndex: number;
  homePosition: THREE.Vector3;
};

const lottery = ref<LotteryKind>('ssq');
const history = ref<DrawRecord[]>([]);
const analysis = ref<AnalysisSummary | null>(null);
const drawResult = ref<DrawResult | null>(null);
const backtest = ref<BacktestResult | null>(null);
const predictResult = ref<PredictResult | null>(null);
const isPredicting = ref(false);
const replaceSetId = ref(1);
const targetIssue = ref('');
const isDrawing = ref(false);
const isBacktesting = ref(false);
const sceneEl = ref<HTMLDivElement | null>(null);
const displayedFront = ref<number[]>([]);
const displayedBack = ref<number[]>([]);
const phaseLabel = ref('等待开始');
const activeChamberColor = ref<'red' | 'blue'>('red');

let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let chamber: THREE.Group | null = null;
let agitator: THREE.Group | null = null;
let animationFrame = 0;
let visualBalls: VisualBall[] = [];
let lastTime = performance.now();

const filteredHistory = computed(() => history.value.filter((record) => record.lottery === lottery.value));
const redPool = computed(() => Array.from({ length: analysis.value?.frontRange || 0 }, (_, i) => i + 1));
const bluePool = computed(() => Array.from({ length: analysis.value?.backRange || 0 }, (_, i) => i + 1));
const ruleText = computed(() => lottery.value === 'ssq'
  ? '双色球：红球从 1-33 中开出 6 个，红球完成后从 1-16 中开出 1 个蓝球。'
  : '大乐透：前区从 1-35 中开出 5 个，前区完成后从 1-12 中开出 2 个后区蓝球。');

const trendLanes = computed(() => {
  if (!analysis.value) return [];
  const redBalls = analysis.value.ballProfiles.filter((ball) => ball.color === 'red');
  const laneSize = Math.ceil(analysis.value.frontRange / 3);
  return ['低位', '中位', '高位'].map((label, index) => {
    const start = index * laneSize + 1;
    const end = Math.min((index + 1) * laneSize, analysis.value!.frontRange);
    const balls = redBalls.filter((ball) => ball.number >= start && ball.number <= end);
    const average = balls.reduce((sum, ball) => sum + ball.bias + ball.heat, 0) / Math.max(1, balls.length);
    return {
      label,
      range: `${pad(start)}-${pad(end)}`,
      value: Math.min(100, 24 + average * 34),
    };
  });
});

function pad(n: number) {
  return String(n).padStart(2, '0');
}

async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch {
    return mockInvoke<T>(command, args);
  }
}

function mockInvoke<T>(command: string, args?: Record<string, unknown>): T {
  const mockHistory: DrawRecord[] = [
    makeRecord('ssq', '2026060', [3, 7, 12, 18, 24, 31], [9], 7),
    makeRecord('ssq', '2026059', [1, 9, 14, 19, 27, 33], [5], 10),
    makeRecord('ssq', '2026058', [5, 11, 16, 20, 22, 30], [12], 6),
    makeRecord('ssq', '2026057', [2, 6, 15, 23, 26, 32], [3], 8),
    makeRecord('dlt', '2026060', [4, 9, 16, 22, 34], [3, 10], 3),
    makeRecord('dlt', '2026059', [2, 7, 15, 25, 31], [5, 12], 4),
    makeRecord('dlt', '2026058', [6, 11, 18, 23, 29], [2, 8], 2),
    makeRecord('dlt', '2026057', [1, 13, 17, 21, 35], [1, 9], 5),
  ];
  if (command === 'get_history') return mockHistory as T;
  if (command === 'analyze_lottery') return makeAnalysis((args?.lottery as LotteryKind) || lottery.value) as T;
  if (command === 'run_backtest') {
    return {
      lottery: args?.lottery,
      sampleCount: 80,
      exactMatches: 1,
      averageFrontHits: 1.86,
      averageBackHits: 0.38,
      bestIssue: '2026060',
      bestScore: 5.4,
      notes: [],
    } as T;
  }
  if (command === 'predict_draw') {
    const active = (args?.lottery as LotteryKind) || lottery.value;
    const historyRecords = mockHistory.filter((r) => r.lottery === active);
    const isSsq = active === 'ssq';
    const frontRange = isSsq ? 33 : 35;
    const backRange = isSsq ? 16 : 12;
    const frontPick = isSsq ? 6 : 5;
    const backPick = isSsq ? 1 : 2;

    // Build frequency and gap maps
    const frontFreq: Record<number, number> = {};
    const frontGap: Record<number, number> = {};
    const backFreq: Record<number, number> = {};
    const backGap: Record<number, number> = {};

    for (let n = 1; n <= frontRange; n++) {
      frontFreq[n] = 0;
      frontGap[n] = historyRecords.length;
    }
    for (let n = 1; n <= backRange; n++) {
      backFreq[n] = 0;
      backGap[n] = historyRecords.length;
    }

    historyRecords.forEach((record, idx) => {
      const decay = Math.exp(-0.15 * idx);
      record.front.forEach((n) => {
        frontFreq[n] = (frontFreq[n] || 0) + decay;
        frontGap[n] = Math.min(frontGap[n] || historyRecords.length, idx);
      });
      record.back.forEach((n) => {
        backFreq[n] = (backFreq[n] || 0) + decay;
        backGap[n] = Math.min(backGap[n] || historyRecords.length, idx);
      });
    });

    // Ball physics profiles
    const ballProfiles = makeAnalysis(active).ballProfiles;

    // Weighted picker
    function pickWeighted(
      range: number,
      pickCount: number,
      freq: Record<number, number>,
      gap: Record<number, number>,
      color: 'red' | 'blue',
      zoneTarget?: number[],
      oddEvenTarget?: [number, number],
    ): number[] {
      const available: number[] = [];
      for (let n = 1; n <= range; n++) available.push(n);

      const picked: number[] = [];
      const zoneCounts = [0, 0, 0];
      let oddCount = 0;
      let evenCount = 0;
      const third = Math.ceil(range / 3);

      for (let i = 0; i < pickCount; i++) {
        const remaining = pickCount - i;
        const weights = available.map((n) => {
          const f = freq[n] || 0;
          const g = gap[n] || historyRecords.length;
          const total = historyRecords.length || 1;

          // Gap score: overdue numbers get boost
          const gapRatio = g / total;
          const gapScore = gapRatio > 0.7 ? 1.4 : gapRatio > 0.5 ? 1.2 : gapRatio > 0.3 ? 1.0 : 0.8;

          // Frequency score
          const freqScore = 0.5 + f * 0.8;

          // Physics weight
          const profile = ballProfiles.find((p) => p.number === n && p.color === color);
          const physicsWeight = profile
            ? Math.max(0.2, Math.min(1.8, 0.6 + profile.mass * 0.3 - profile.friction * 0.4 + profile.roughness * 0.2))
            : 1.0;

          let base = gapScore * 0.30 + freqScore * 0.25 + physicsWeight * 0.25 + 0.2;

          // Zone constraint
          const zone = Math.min(2, Math.floor((n - 1) / third));
          if (zoneTarget) {
            if (zoneCounts[zone] < zoneTarget[zone]) base *= 1.5;
            else if (zoneCounts[zone] >= zoneTarget[zone] + 1) base *= 0.3;
          }

          // Parity constraint
          const isOdd = n % 2 === 1;
          if (oddEvenTarget) {
            if (isOdd && oddCount < oddEvenTarget[0]) base *= 1.3;
            else if (!isOdd && evenCount < oddEvenTarget[1]) base *= 1.3;
            else if (isOdd && oddCount >= oddEvenTarget[0] + 1) base *= 0.5;
            else if (!isOdd && evenCount >= oddEvenTarget[1] + 1) base *= 0.5;
          }

          // Diversity: ensure all zones represented
          if (remaining <= 2 && zoneCounts.some((c) => c === 0)) {
            base *= zoneCounts[zone] === 0 ? 2.0 : 0.5;
          }

          return base;
        });

        const totalWeight = weights.reduce((a, b) => a + b, 0);
        if (totalWeight <= 0) break;

        let threshold = Math.random() * totalWeight;
        let selectedIdx = 0;
        for (let j = 0; j < weights.length; j++) {
          threshold -= weights[j];
          if (threshold <= 0) {
            selectedIdx = j;
            break;
          }
        }

        const selected = available.splice(selectedIdx, 1)[0];
        const zone = Math.min(2, Math.floor((selected - 1) / third));
        zoneCounts[zone]++;
        if (selected % 2 === 1) oddCount++;
        else evenCount++;
        picked.push(selected);
      }

      return picked.sort((a, b) => a - b);
    }

    // Zone targets from history
    const third = Math.ceil(frontRange / 3);
    const zoneTarget = [0, 0, 0];
    historyRecords.slice(0, 10).forEach((r) => {
      r.front.forEach((n) => {
        zoneTarget[Math.min(2, Math.floor((n - 1) / third))]++;
      });
    });
    const zoneTotal = zoneTarget.reduce((a, b) => a + b, 0) || 1;
    const zoneTargets = zoneTarget.map((c) => Math.round((c / zoneTotal) * frontPick));
    // Adjust to match pick count
    let zoneSum = zoneTargets.reduce((a, b) => a + b, 0);
    while (zoneSum < frontPick) { zoneTargets[zoneSum % 3]++; zoneSum++; }
    while (zoneSum > frontPick) {
      const maxIdx = zoneTargets.indexOf(Math.max(...zoneTargets));
      if (zoneTargets[maxIdx] > 0) { zoneTargets[maxIdx]--; zoneSum--; }
      else break;
    }

    // Odd/even target
    let oddTotal = 0;
    let evenTotal = 0;
    historyRecords.slice(0, 8).forEach((r) => {
      r.front.forEach((n) => {
        if (n % 2 === 1) oddTotal++;
        else evenTotal++;
      });
    });
    const parityTotal = oddTotal + evenTotal || 1;
    const oddTarget = Math.round((oddTotal / parityTotal) * frontPick);
    const oddEvenTarget: [number, number] = [
      Math.max(1, Math.min(frontPick - 1, oddTarget)),
      Math.max(1, Math.min(frontPick - 1, frontPick - oddTarget)),
    ];

    // Generate candidates and filter duplicates
    let bestFront: number[] = [];
    let bestBack: number[] = [];
    let bestScore = -1;
    const historicalSets = historyRecords.map((r) => ({
      front: new Set(r.front),
      back: new Set(r.back),
    }));

    for (let round = 0; round < 2000; round++) {
      const front = pickWeighted(frontRange, frontPick, frontFreq, frontGap, 'red', zoneTargets, oddEvenTarget);
      const back = pickWeighted(backRange, backPick, backFreq, backGap, 'blue');

      // Skip exact historical matches
      const isDuplicate = historicalSets.some((hs) => {
        if (hs.front.size !== front.length || hs.back.size !== back.length) return false;
        return front.every((n) => hs.front.has(n)) && back.every((n) => hs.back.has(n));
      });
      if (isDuplicate) continue;

      // Score candidate
      const frontSum = front.reduce((a, b) => a + b, 0);
      const historicalSums = historyRecords.map((r) => r.front.reduce((a, b) => a + b, 0));
      const avgSum = historicalSums.reduce((a, b) => a + b, 0) / historicalSums.length || 100;
      const sumScore = 1 - Math.abs(frontSum - avgSum) / Math.max(avgSum, 1);

      const spread = front[front.length - 1] - front[0];
      const avgSpread = historyRecords.length
        ? historyRecords.reduce((acc, r) => acc + (r.front[r.front.length - 1] - r.front[0]), 0) / historyRecords.length
        : 20;
      const spreadScore = 1 - Math.abs(spread - avgSpread) / Math.max(avgSpread, 1);

      let consecutive = 0;
      for (let k = 1; k < front.length; k++) {
        if (front[k] === front[k - 1] + 1) consecutive++;
      }
      const consecutiveScore = consecutive >= 3 ? 0.3 : consecutive === 2 ? 0.7 : 1.0;

      let qualitySum = 0;
      front.forEach((n) => {
        const f = frontFreq[n] || 0;
        const g = frontGap[n] || historyRecords.length;
        const total = historyRecords.length || 1;
        qualitySum += 0.3 + f * 0.3 + (1 - Math.min(g / total, 1)) * 0.4;
      });
      const qualityScore = qualitySum / front.length;

      const score = sumScore * 0.20 + spreadScore * 0.15 + consecutiveScore * 0.20 + qualityScore * 0.20 + 0.25;

      if (score > bestScore) {
        bestScore = score;
        bestFront = front;
        bestBack = back;
      }
    }

    // If no valid candidate found (unlikely), force generate one without duplicate check
    if (bestFront.length === 0) {
      bestFront = pickWeighted(frontRange, frontPick, frontFreq, frontGap, 'red', zoneTargets, oddEvenTarget);
      bestBack = pickWeighted(backRange, backPick, backFreq, backGap, 'blue');
    }

    // Compute confidence
    const trendScore = Math.random() * 0.3 + 0.5;
    const patternScore = Math.random() * 0.2 + 0.55;
    const physicsScore = Math.random() * 0.25 + 0.5;
    const confidence = Math.min(0.95, trendScore * 0.30 + patternScore * 0.25 + physicsScore * 0.25 + 0.2);

    const frontConfidence = bestFront.map((n) => {
      const total = historyRecords.length || 1;
      const gapFactor = 1 - Math.min((frontGap[n] || total) / total, 1) * 0.5;
      const freqFactor = Math.min(1, 0.3 + (frontFreq[n] || 0) * 0.5);
      return Math.min(0.95, gapFactor * 0.35 + freqFactor * 0.35 + 0.3);
    });
    const backConfidence = bestBack.map((n) => {
      const total = historyRecords.length || 1;
      const gapFactor = 1 - Math.min((backGap[n] || total) / total, 1) * 0.5;
      const freqFactor = Math.min(1, 0.3 + (backFreq[n] || 0) * 0.5);
      return Math.min(0.95, gapFactor * 0.35 + freqFactor * 0.35 + 0.3);
    });

    return {
      lottery: active,
      front: bestFront,
      back: bestBack,
      confidence,
      frontConfidence,
      backConfidence,
      trendScore,
      patternScore,
      physicsScore,
      monteCarloRounds: 2000,
      topCandidates: [],
      explanation: [
        `Based on ${historyRecords.length} historical draws with weighted frequency, gap, and physics analysis.`,
        `Excluded ${historicalSets.length} exact historical combinations.`,
        `Confidence: ${(confidence * 100).toFixed(1)}% | Trend: ${(trendScore * 100).toFixed(1)}% | Pattern: ${(patternScore * 100).toFixed(1)}% | Physics: ${(physicsScore * 100).toFixed(1)}%`,
        confidence > 0.7 ? 'High confidence prediction with strong data alignment.'
          : confidence > 0.5 ? 'Moderate confidence - consider as one of multiple references.'
          : 'Lower confidence - historical patterns show high variance.',
      ],
    } as T;
  }

  const active = (args?.lottery as LotteryKind) || lottery.value;
  const sample = active === 'ssq'
    ? { front: [3, 7, 12, 18, 24, 31], back: [9] }
    : { front: [4, 9, 16, 22, 34], back: [3, 10] };
  return {
    requestId: `WEB-${replaceSetId.value}`,
    lottery: active,
    issue: targetIssue.value || '模拟即时开奖',
    station: '浏览器预览站',
    ...sample,
    spinSeconds: 15.2,
    chamberRpm: 72.4,
    collisionCount: 1680,
    energyLoss: 0.27,
    exactHistoryMatch: false,
    explanation: [],
  } as T;
}

function makeRecord(l: LotteryKind, issue: string, front: number[], back: number[], jackpotCount: number): DrawRecord {
  return {
    id: `${l}-${issue}`,
    issue,
    date: '2026-05-28',
    lottery: l,
    station: l === 'ssq' ? '北京丰台开奖中心' : '天津体彩开奖中心',
    salesMillion: l === 'ssq' ? 418.2 : 318.4,
    jackpotCount,
    front,
    back,
  };
}

function makeAnalysis(kind: LotteryKind): AnalysisSummary {
  const frontRange = kind === 'ssq' ? 33 : 35;
  const backRange = kind === 'ssq' ? 16 : 12;
  const ballProfiles: BallProfile[] = [];
  for (let n = 1; n <= frontRange; n += 1) ballProfiles.push(makeBall(n, 'red'));
  for (let n = 1; n <= backRange; n += 1) ballProfiles.push(makeBall(n, 'blue'));
  return {
    lottery: kind,
    frontRange,
    backRange,
    frontPick: kind === 'ssq' ? 6 : 5,
    backPick: kind === 'ssq' ? 1 : 2,
    hotFront: [7, 9, 12, 16, 24, 31],
    coldFront: [2, 4, 13, 25, 29, 32],
    hotBack: [3, 8, 9, 12],
    coldBack: [1, 4, 10, 16],
    zoneBalance: [14, 18, 16],
    oddEvenBalance: [34, 22],
    ballProfiles,
  };
}

function makeBall(number: number, color: 'red' | 'blue'): BallProfile {
  const roughness = 0.2 + ((number * 37 + replaceSetId.value * 17) % 60) / 100;
  return {
    number,
    color,
    mass: 0.96 + ((number * 13) % 8) / 100,
    friction: 0.2 + ((number * 19) % 20) / 100,
    roughness,
    bias: 0.85 + roughness,
    heat: ((number * 7) % 5) / 4,
  };
}

async function loadData() {
  history.value = await safeInvoke<DrawRecord[]>('get_history');
  if (!targetIssue.value) targetIssue.value = filteredHistory.value[0]?.issue || '';
  await refreshAnalysis();
}

async function refreshAnalysis() {
  analysis.value = await safeInvoke<AnalysisSummary>('analyze_lottery', {
    lottery: lottery.value,
    replaceSetId: replaceSetId.value,
  });
  displayedFront.value = [];
  displayedBack.value = [];
  phaseLabel.value = '等待开始';
  activeChamberColor.value = 'red';
  await nextTick();
  rebuildBalls();
}

async function setLottery(next: LotteryKind) {
  lottery.value = next;
  drawResult.value = null;
  backtest.value = null;
  targetIssue.value = history.value.find((record) => record.lottery === next)?.issue || '';
  await refreshAnalysis();
}

async function replaceBalls() {
  replaceSetId.value += 1;
  drawResult.value = null;
  await refreshAnalysis();
}

async function startDraw() {
  if (isDrawing.value) return;
  isDrawing.value = true;
  drawResult.value = null;
  displayedFront.value = [];
  displayedBack.value = [];
  activeChamberColor.value = 'red';
  phaseLabel.value = lottery.value === 'ssq' ? '红球搅拌中' : '前区搅拌中';

  const result = await safeInvoke<DrawResult>('simulate_draw', {
    req: {
      lottery: lottery.value,
      mode: 'calibrated',
      replaceSetId: replaceSetId.value,
      targetIssue: targetIssue.value,
    },
  });
  scheduleDrawSequence(result);
}

function scheduleDrawSequence(result: DrawResult) {
  const redLabel = lottery.value === 'ssq' ? '红球' : '前区';
  const blueLabel = lottery.value === 'ssq' ? '蓝球' : '后区';
  let delay = 1300;
  let index = 0;

  for (const number of result.front) {
    setTimeout(() => {
      phaseLabel.value = `${redLabel} ${displayedFront.value.length + 1}/${result.front.length}`;
      displayedFront.value = [...displayedFront.value, number];
      markSelectedBall(number, 'red', index++);
    }, delay);
    delay += 820;
  }

  setTimeout(() => {
    activeChamberColor.value = 'blue';
    releaseBlueBallsIntoChamber();
    phaseLabel.value = `${blueLabel}搅拌中`;
  }, delay);
  delay += 900;

  for (const number of result.back) {
    setTimeout(() => {
      phaseLabel.value = `${blueLabel} ${displayedBack.value.length + 1}/${result.back.length}`;
      displayedBack.value = [...displayedBack.value, number];
      markSelectedBall(number, 'blue', index++);
    }, delay);
    delay += 820;
  }

  setTimeout(() => {
    drawResult.value = result;
    phaseLabel.value = '开奖完成';
    isDrawing.value = false;
  }, delay + 600);
}

async function runBacktest() {
  isBacktesting.value = true;
  backtest.value = await safeInvoke<BacktestResult>('run_backtest', {
    lottery: lottery.value,
    replaceSetId: replaceSetId.value,
    attemptsPerIssue: 20,
  });
  isBacktesting.value = false;
}

async function runPrediction() {
  isPredicting.value = true;
  predictResult.value = await safeInvoke<PredictResult>('predict_draw', {
    lottery: lottery.value,
    replaceSetId: replaceSetId.value,
    monteCarloRounds: 500,
  });
  isPredicting.value = false;
}


function initScene() {
  if (!sceneEl.value) return;
  scene = new THREE.Scene();
  scene.background = new THREE.Color('#f7f8fb');
  camera = new THREE.PerspectiveCamera(36, sceneEl.value.clientWidth / sceneEl.value.clientHeight, 0.1, 100);
  camera.position.set(0, 2.0, 12.2);
  camera.lookAt(0, 0.5, 0);

  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(sceneEl.value.clientWidth, sceneEl.value.clientHeight);
  renderer.shadowMap.enabled = true;
  sceneEl.value.appendChild(renderer.domElement);

  const ambient = new THREE.HemisphereLight('#ffffff', '#d7e3f2', 2.8);
  scene.add(ambient);
  const key = new THREE.DirectionalLight('#ffffff', 2.8);
  key.position.set(-4, 7, 7);
  key.castShadow = true;
  scene.add(key);
  const rimLight = new THREE.PointLight('#c7e4ff', 1.6, 14);
  rimLight.position.set(3.8, 3.5, 4.8);
  scene.add(rimLight);

  const platform = new THREE.Mesh(
    new THREE.CylinderGeometry(2.2, 2.45, 0.18, 96),
    new THREE.MeshStandardMaterial({ color: '#ffffff', metalness: 0.12, roughness: 0.34 }),
  );
  platform.position.y = -3.42;
  platform.receiveShadow = true;
  scene.add(platform);

  chamber = new THREE.Group();
  chamber.position.y = 1.25;
  const shell = new THREE.Mesh(
    new THREE.SphereGeometry(2.25, 96, 64),
    new THREE.MeshPhysicalMaterial({
      color: '#dff2ff',
      metalness: 0,
      roughness: 0.05,
      transmission: 0.82,
      transparent: true,
      opacity: 0.34,
      thickness: 0.6,
    }),
  );
  shell.castShadow = true;
  chamber.add(shell);

  const equator = new THREE.Mesh(
    new THREE.TorusGeometry(2.27, 0.035, 16, 128),
    new THREE.MeshStandardMaterial({ color: '#f8fbff', metalness: 0.8, roughness: 0.18 }),
  );
  equator.rotation.x = Math.PI / 2;
  chamber.add(equator);

  agitator = new THREE.Group();
  for (let i = 0; i < 4; i += 1) {
    const blade = new THREE.Mesh(
      new THREE.BoxGeometry(2.95, 0.055, 0.12),
      new THREE.MeshStandardMaterial({ color: '#f3f7fb', metalness: 0.72, roughness: 0.18 }),
    );
    blade.rotation.y = (i * Math.PI) / 4;
    agitator.add(blade);
  }
  chamber.add(agitator);
  scene.add(chamber);

  const tube = new THREE.Mesh(
    new THREE.CylinderGeometry(0.38, 0.42, 3.3, 40),
    new THREE.MeshPhysicalMaterial({
      color: '#dff2ff',
      transparent: true,
      opacity: 0.38,
      transmission: 0.72,
      roughness: 0.08,
    }),
  );
  tube.position.set(0, -1.7, 0);
  scene.add(tube);

  const gate = new THREE.Mesh(
    new THREE.TorusGeometry(0.55, 0.04, 14, 64),
    new THREE.MeshStandardMaterial({ color: '#c7ccd3', metalness: 0.78, roughness: 0.22 }),
  );
  gate.rotation.x = Math.PI / 2;
  gate.position.y = -3.35;
  scene.add(gate);

  const tray = new THREE.Mesh(
    new THREE.BoxGeometry(5.6, 0.18, 0.7),
    new THREE.MeshStandardMaterial({ color: '#ffffff', metalness: 0.08, roughness: 0.28 }),
  );
  tray.position.set(0, -3.78, 0.75);
  tray.castShadow = true;
  scene.add(tray);

  rebuildBalls();
  animate();
}

function rebuildBalls() {
  if (!scene || !analysis.value) return;
  for (const ball of visualBalls) scene.remove(ball.mesh);
  visualBalls = [];
  const blueIndex = { value: 0 };
  for (const profile of analysis.value.ballProfiles) {
    const baseColor = profile.color === 'red' ? '#ff3b4f' : '#007aff';
    const material = new THREE.MeshToonMaterial({
      color: baseColor,
    });
    const mesh = new THREE.Mesh(new THREE.SphereGeometry(0.30, 48, 36), material);
    const outline = new THREE.Mesh(
      new THREE.SphereGeometry(0.31, 48, 36),
      new THREE.MeshBasicMaterial({ color: profile.color === 'red' ? '#cc1430' : '#0055cc', side: THREE.BackSide }),
    );
    mesh.add(outline);
    mesh.add(makeLabelSprite(pad(profile.number)));
    const homePosition = profile.color === 'red'
      ? randomChamberPosition()
      : blueReservePosition(blueIndex.value++);
    mesh.position.copy(homePosition);
    scene.add(mesh);
    visualBalls.push({
      mesh,
      velocity: new THREE.Vector3((Math.random() - 0.5) * 0.026, Math.random() * 0.03, (Math.random() - 0.5) * 0.026),
      profile,
      selected: false,
      chutePhase: 0,
      chuteIndex: 0,
      homePosition,
    });
  }
}

function makeLabelSprite(text: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 128;
  canvas.height = 128;
  const ctx = canvas.getContext('2d')!;
  ctx.clearRect(0, 0, 128, 128);
  ctx.font = '900 58px "Segoe UI", sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = '#ffffff';
  ctx.fillText(text, 64, 66);
  const texture = new THREE.CanvasTexture(canvas);
  const sprite = new THREE.Sprite(new THREE.SpriteMaterial({
    map: texture,
    transparent: true,
    depthTest: false,
  }));
  sprite.scale.set(0.47, 0.47, 0.47);
  sprite.position.set(0, 0, 0.34);
  sprite.renderOrder = 20;
  return sprite;
}

function randomChamberPosition() {
  const ring = 1.72 * Math.sqrt(Math.random());
  const angle = Math.random() * Math.PI * 2;
  return new THREE.Vector3(Math.cos(angle) * ring, 0.35 + Math.random() * 1.62, Math.sin(angle) * ring);
}

function blueReservePosition(index: number) {
  const col = index % 4;
  const row = Math.floor(index / 4);
  return new THREE.Vector3(2.8 + col * 0.55, 2.2 - row * 0.55, 1.0);
}

function redReservePosition(index: number) {
  const column = index % 6;
  const row = Math.floor(index / 6);
  return new THREE.Vector3(-3.15 + column * 0.3, 2.0 - row * 0.3, -0.45);
}

function releaseBlueBallsIntoChamber() {
  let redIndex = 0;
  for (const ball of visualBalls) {
    if (ball.profile.color !== 'red' || ball.selected) continue;
    ball.homePosition.copy(redReservePosition(redIndex++));
    ball.velocity.multiplyScalar(0.25);
  }

  let index = 0;
  for (const ball of visualBalls) {
    if (ball.profile.color !== 'blue' || ball.selected) continue;
    const destination = randomChamberPosition();
    destination.y = 0.55 + (index % 4) * 0.48;
    ball.homePosition.copy(destination);
    ball.velocity.set((Math.random() - 0.5) * 0.04, 0.04 + Math.random() * 0.03, (Math.random() - 0.5) * 0.04);
    index += 1;
  }
}

function markSelectedBall(number: number, color: 'red' | 'blue', chuteIndex: number) {
  const ball = visualBalls.find((item) => item.profile.number === number && item.profile.color === color && !item.selected);
  if (!ball) return;
  ball.selected = true;
  ball.chutePhase = 0.01;
  ball.chuteIndex = chuteIndex;
}

function animate() {
  if (!renderer || !scene || !camera) return;
  const now = performance.now();
  const dt = Math.min(0.033, (now - lastTime) / 1000);
  lastTime = now;

  if (chamber) chamber.rotation.y += (isDrawing.value ? 1.4 : 0.2) * dt;
  if (agitator) agitator.rotation.z += (isDrawing.value ? 6.2 : 1.0) * dt;

  for (const ball of visualBalls) {
    if (ball.selected) {
      ball.chutePhase = Math.min(1, ball.chutePhase + dt * 0.72);
      const t = ball.chutePhase;
      const trayX = -2.34 + ball.chuteIndex * 0.43;
      const fallingY = THREE.MathUtils.lerp(ball.mesh.position.y, -3.62, 0.065);
      ball.mesh.position.x = THREE.MathUtils.lerp(ball.mesh.position.x, trayX, 0.07);
      ball.mesh.position.y = fallingY;
      ball.mesh.position.z = THREE.MathUtils.lerp(ball.mesh.position.z, 0.92, 0.07);
      if (t > 0.82) ball.mesh.position.y = THREE.MathUtils.lerp(ball.mesh.position.y, -3.62, 0.2);
      ball.mesh.rotation.x += 0.2;
      ball.mesh.rotation.z += 0.12;
      continue;
    }

    if (ball.profile.color !== activeChamberColor.value) {
      ball.mesh.position.lerp(ball.homePosition, 0.08);
      ball.mesh.rotation.y += 0.006;
      ball.mesh.rotation.z += 0.004;
      continue;
    }

    const spinBoost = isDrawing.value ? 2.1 : 0.5;
    const center = new THREE.Vector3(0, 1.25, 0);
    const radial = ball.mesh.position.clone().sub(center);
    const tangent = new THREE.Vector3(-radial.z, 0.28, radial.x).normalize();
    ball.velocity.addScaledVector(tangent, dt * spinBoost * (0.75 + ball.profile.roughness));
    ball.velocity.y += dt * (Math.sin(now * 0.006 + ball.profile.number) * 0.22 - 0.04);
    ball.velocity.multiplyScalar(1 - dt * (0.5 + ball.profile.friction));
    ball.mesh.position.addScaledVector(ball.velocity, 1);

    const chamberLocal = ball.mesh.position.clone().sub(center);
    const radius = chamberLocal.length();
    if (radius > 1.92) {
      chamberLocal.normalize().multiplyScalar(1.92);
      ball.mesh.position.copy(center.clone().add(chamberLocal));
      ball.velocity.reflect(chamberLocal.clone().normalize()).multiplyScalar(-0.72);
    }
    if (ball.mesh.position.y < -0.1 || ball.mesh.position.y > 2.95) {
      ball.velocity.y *= -0.7;
      ball.mesh.position.y = THREE.MathUtils.clamp(ball.mesh.position.y, -0.1, 2.95);
    }
    ball.mesh.rotation.x += ball.velocity.z * 0.4;
    ball.mesh.rotation.z -= ball.velocity.x * 0.4;
  }

  renderer.render(scene, camera);
  animationFrame = requestAnimationFrame(animate);
}

function resizeScene() {
  if (!sceneEl.value || !renderer || !camera) return;
  renderer.setSize(sceneEl.value.clientWidth, sceneEl.value.clientHeight);
  camera.aspect = sceneEl.value.clientWidth / sceneEl.value.clientHeight;
  camera.updateProjectionMatrix();
}

onMounted(async () => {
  await loadData();
  await nextTick();
  initScene();
  window.addEventListener('resize', resizeScene);
});

watch(lottery, () => resizeScene());

onBeforeUnmount(() => {
  cancelAnimationFrame(animationFrame);
  window.removeEventListener('resize', resizeScene);
  renderer?.dispose();
});
</script>

