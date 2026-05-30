use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawRecord {
    pub id: String,
    pub issue: String,
    pub date: String,
    pub lottery: LotteryKind,
    pub station: String,
    pub sales_million: f64,
    pub jackpot_count: u32,
    pub front: Vec<u8>,
    pub back: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum LotteryKind {
    Ssq,
    Dlt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BallProfile {
    pub number: u8,
    pub color: String,
    pub mass: f64,
    pub friction: f64,
    pub roughness: f64,
    pub bias: f64,
    pub heat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub lottery: LotteryKind,
    pub front_range: u8,
    pub back_range: u8,
    pub front_pick: u8,
    pub back_pick: u8,
    pub hot_front: Vec<u8>,
    pub cold_front: Vec<u8>,
    pub hot_back: Vec<u8>,
    pub cold_back: Vec<u8>,
    pub zone_balance: Vec<u32>,
    pub odd_even_balance: Vec<u32>,
    pub ball_profiles: Vec<BallProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawRequest {
    pub lottery: LotteryKind,
    pub mode: String,
    pub seed: Option<u64>,
    pub replace_set_id: u32,
    pub target_issue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawResult {
    pub request_id: String,
    pub lottery: LotteryKind,
    pub issue: String,
    pub station: String,
    pub front: Vec<u8>,
    pub back: Vec<u8>,
    pub spin_seconds: f64,
    pub chamber_rpm: f64,
    pub collision_count: u32,
    pub energy_loss: f64,
    pub exact_history_match: bool,
    pub explanation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResult {
    pub lottery: LotteryKind,
    pub sample_count: u32,
    pub exact_matches: u32,
    pub average_front_hits: f64,
    pub average_back_hits: f64,
    pub best_issue: String,
    pub best_score: f64,
    pub notes: Vec<String>,
}

// ======================== ????????? ========================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictRequest {
    pub lottery: LotteryKind,
    pub replace_set_id: u32,
    pub monte_carlo_rounds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictResult {
    pub lottery: LotteryKind,
    pub front: Vec<u8>,
    pub back: Vec<u8>,
    pub confidence: f64,
    pub front_confidence: Vec<f64>,
    pub back_confidence: Vec<f64>,
    pub trend_score: f64,
    pub pattern_score: f64,
    pub physics_score: f64,
    pub monte_carlo_rounds: u32,
    pub top_candidates: Vec<CandidateSet>,
    pub explanation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateSet {
    pub front: Vec<u8>,
    pub back: Vec<u8>,
    pub composite_score: f64,
}


// ======================== ???????? ========================

#[tauri::command]
fn predict_draw(req: PredictRequest) -> PredictResult {
    let rounds = req.monte_carlo_rounds.unwrap_or(2000);
    let analysis = build_analysis(req.lottery, req.replace_set_id);
    let records: Vec<DrawRecord> = history()
        .into_iter()
        .filter(|r| r.lottery == req.lottery)
        .collect();

    let front_range = analysis.front_range;
    let back_range = analysis.back_range;
    let front_pick = analysis.front_pick as usize;
    let back_pick = analysis.back_pick as usize;

    // Build historical sets for fast duplicate checking
    let historical_sets: Vec<(HashSet<u8>, HashSet<u8>)> = records.iter()
        .map(|r| {
            let f: HashSet<u8> = r.front.iter().copied().collect();
            let b: HashSet<u8> = r.back.iter().copied().collect();
            (f, b)
        })
        .collect();

    // Compute zone and parity targets from history
    let zone_targets = compute_zone_targets_usize(&records, front_range, front_pick);
    let odd_even_target = compute_odd_even_target(&records, front_pick);

    // Build number stats with physics
    let front_profiles: Vec<&BallProfile> = analysis.ball_profiles.iter()
        .filter(|b| b.color == "red").collect();
    let back_profiles: Vec<&BallProfile> = analysis.ball_profiles.iter()
        .filter(|b| b.color != "red").collect();

    let front_stats = build_number_stats(&records, &front_profiles, front_range, true);
    let back_stats = build_number_stats(&records, &back_profiles, back_range, false);

    // Collect hot/cold sets
    let hot_front: HashSet<u8> = analysis.hot_front.iter().copied().collect();
    let cold_front: HashSet<u8> = analysis.cold_front.iter().copied().collect();
    let hot_back: HashSet<u8> = analysis.hot_back.iter().copied().collect();
    let cold_back: HashSet<u8> = analysis.cold_back.iter().copied().collect();

    let mut rng = ChaCha8Rng::seed_from_u64(
        (req.replace_set_id as u64).wrapping_mul(6364136223846793005)
            .wrapping_add(records.len() as u64)
    );

    let mut candidates: Vec<CandidateSet> = Vec::new();
    let mut attempts = 0usize;
    let max_attempts = rounds * 50;

    while candidates.len() < rounds as usize && attempts < max_attempts as usize {
        attempts += 1;

        let front = pick_numbers_advanced(
            &mut rng, &front_stats, front_pick, &zone_targets, odd_even_target,
            &hot_front, &cold_front, records.len(),
        );
        let back = pick_numbers_advanced(
            &mut rng, &back_stats, back_pick, &vec![1, 1], (1, 1),
            &hot_back, &cold_back, records.len(),
        );

        // CRITICAL: Skip exact historical matches
        let front_set: HashSet<u8> = front.iter().copied().collect();
        let back_set: HashSet<u8> = back.iter().copied().collect();
        let is_duplicate = historical_sets.iter().any(|(hf, hb)| {
            hf == &front_set && hb == &back_set
        });
        if is_duplicate { continue; }

        let score = evaluate_candidate_advanced(
            &front, &back, &front_stats, &back_stats, &records,
        );

        candidates.push(CandidateSet {
            front: front.clone(),
            back: back.clone(),
            composite_score: score,
        });
    }

    // Sort by score descending
    candidates.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(20);

    let best = &candidates[0];
    let top5_scores: Vec<f64> = candidates.iter().take(5).map(|c| c.composite_score).collect();
    let avg_top = top5_scores.iter().sum::<f64>() / top5_scores.len().max(1) as f64;
    let score_std = (top5_scores.iter().map(|s| (s - avg_top).powi(2)).sum::<f64>() / top5_scores.len().max(1) as f64).sqrt();

    let trend_score = compute_trend_alignment(&best.front, &best.back, &records);
    let pattern_score = evaluate_pattern_fit(&best.front, &best.back, &records, front_range);
    let physics_score = evaluate_physics_fit(&best.front, &best.back, &analysis);
    let monte_carlo_stability = 1.0 / (1.0 + score_std);
    let confidence = (trend_score * 0.30 + pattern_score * 0.25 + physics_score * 0.25 + monte_carlo_stability * 0.20)
        .clamp(0.0, 1.0);

    let front_confidence: Vec<f64> = best.front.iter().map(|&n| {
        compute_number_confidence_advanced(n, &front_stats, records.len())
    }).collect();
    let back_confidence: Vec<f64> = best.back.iter().map(|&n| {
        compute_number_confidence_advanced(n, &back_stats, records.len())
    }).collect();

    let mut explanation = vec![
        format!("Based on {} historical draws with weighted frequency, gap, and physics analysis.", records.len()),
        format!("Excluded {} exact historical combinations. Monte Carlo: {} rounds, {} attempts.",
            historical_sets.len(), candidates.len(), attempts),
        format!("Confidence: {:.1}% | Trend: {:.1}% | Pattern: {:.1}% | Physics: {:.1}%",
            confidence * 100.0, trend_score * 100.0, pattern_score * 100.0, physics_score * 100.0),
    ];

    if confidence > 0.7 {
        explanation.push("High confidence prediction with strong data alignment.".to_string());
    } else if confidence > 0.5 {
        explanation.push("Moderate confidence - consider as one of multiple references.".to_string());
    } else {
        explanation.push("Lower confidence - historical patterns show high variance.".to_string());
    }

    PredictResult {
        lottery: req.lottery,
        front: best.front.clone(),
        back: best.back.clone(),
        confidence: round3(confidence),
        front_confidence: front_confidence.into_iter().map(round3).collect(),
        back_confidence: back_confidence.into_iter().map(round3).collect(),
        trend_score: round3(trend_score),
        pattern_score: round3(pattern_score),
        physics_score: round3(physics_score),
        monte_carlo_rounds: candidates.len() as u32,
        top_candidates: candidates,
        explanation,
    }
}

// ---- Number Stats Builder ----
fn build_number_stats(
    records: &[DrawRecord],
    pool: &[&BallProfile],
    range: u8,
    is_front: bool,
) -> Vec<NumberStat> {
    let mut stats = Vec::new();
    let total_records = records.len().max(1) as f64;

    for &profile in pool {
        let n = profile.number;
        if n > range { continue; }

        let mut weighted_freq = 0.0;
        let mut last_seen: Option<usize> = None;
        let mut gaps = Vec::new();
        let mut prev_idx: Option<usize> = None;

        for (idx, record) in records.iter().enumerate() {
            let numbers = if is_front { &record.front } else { &record.back };
            let decay = (-0.12 * idx as f64).exp();
            if numbers.contains(&n) {
                weighted_freq += decay;
                if let Some(pi) = prev_idx {
                    gaps.push(idx - pi);
                }
                prev_idx = Some(idx);
                last_seen = Some(idx);
            }
        }

        let last_gap = last_seen.unwrap_or(records.len());
        let avg_gap = if gaps.is_empty() {
            total_records
        } else {
            gaps.iter().sum::<usize>() as f64 / gaps.len() as f64
        };

        let streak_score = if gaps.len() >= 2 {
            let recent_gaps: Vec<usize> = gaps.iter().rev().take(3).copied().collect();
            let avg_recent = recent_gaps.iter().sum::<usize>() as f64 / recent_gaps.len() as f64;
            if avg_recent < 2.5 { 1.3 } else if avg_recent < 4.0 { 1.1 } else { 0.9 }
        } else {
            1.0
        };

        let third = (range as f64 / 3.0).ceil() as u8;
        let zone = ((n - 1) / third).min(2);
        let physics_weight = (0.6 + profile.mass * 0.3 - profile.friction * 0.4 + profile.roughness * 0.2)
            .clamp(0.2, 1.8);

        stats.push(NumberStat {
            number: n,
            frequency: weighted_freq,
            last_gap,
            avg_gap,
            streak_score,
            zone,
            is_odd: n % 2 == 1,
            physics_weight,
        });
    }
    stats
}

// ---- Advanced Number Picker ----
fn pick_numbers_advanced(
    rng: &mut ChaCha8Rng,
    stats: &[NumberStat],
    pick_count: usize,
    zone_targets: &[usize],
    odd_even_target: (usize, usize),
    hot_numbers: &HashSet<u8>,
    cold_numbers: &HashSet<u8>,
    total_records: usize,
) -> Vec<u8> {
    let mut available: Vec<&NumberStat> = stats.iter().collect();
    let mut picked: Vec<u8> = Vec::new();
    let mut zone_counts = [0usize; 3];
    let mut odd_count = 0usize;
    let mut even_count = 0usize;

    for i in 0..pick_count {
        let remaining = pick_count - i;

        let weights: Vec<f64> = available.iter().map(|&s| {
            let total = total_records.max(1) as f64;
            let gap_ratio = s.last_gap as f64 / total;
            let gap_score = if gap_ratio > 0.7 { 1.4 } else if gap_ratio > 0.5 { 1.2 } else if gap_ratio > 0.3 { 1.0 } else { 0.8 };
            let freq_score = 0.5 + s.frequency * 0.8;
            let hot_cold = if hot_numbers.contains(&s.number) { 1.15 }
                else if cold_numbers.contains(&s.number) { 1.25 }
                else { 1.0 };
            let base = gap_score * 0.25 + freq_score * 0.20 + hot_cold * 0.20 + s.physics_weight * 0.20 + s.streak_score * 0.15;

            let zone_boost = if zone_counts[s.zone as usize] < zone_targets.get(s.zone as usize).copied().unwrap_or(1) {
                1.5
            } else if zone_counts[s.zone as usize] >= zone_targets.get(s.zone as usize).copied().unwrap_or(1) + 1 {
                0.3
            } else {
                1.0
            };

            let parity_boost = if s.is_odd && odd_count < odd_even_target.0 {
                1.3
            } else if !s.is_odd && even_count < odd_even_target.1 {
                1.3
            } else if s.is_odd && odd_count >= odd_even_target.0 + 1 {
                0.5
            } else if !s.is_odd && even_count >= odd_even_target.1 + 1 {
                0.5
            } else {
                1.0
            };

            let diversity = if remaining <= 2 && zone_counts.iter().any(|&c| c == 0) {
                if zone_counts[s.zone as usize] == 0 { 2.0 } else { 0.5 }
            } else {
                1.0
            };

            base * zone_boost * parity_boost * diversity
        }).collect();

        let total_weight: f64 = weights.iter().sum();
        if total_weight <= 0.0 { break; }

        let threshold = rng.random::<f64>() * total_weight;
        let mut cumsum = 0.0;
        let mut selected_idx = 0;
        for (j, &w) in weights.iter().enumerate() {
            cumsum += w;
            if cumsum >= threshold {
                selected_idx = j;
                break;
            }
        }

        let selected = available.remove(selected_idx);
        zone_counts[selected.zone as usize] += 1;
        if selected.is_odd { odd_count += 1; } else { even_count += 1; }
        picked.push(selected.number);
    }

    picked.sort();
    picked
}

// ---- Candidate Evaluator ----
fn evaluate_candidate_advanced(
    front: &[u8], back: &[u8],
    front_stats: &[NumberStat],
    _back_stats: &[NumberStat],
    records: &[DrawRecord],
) -> f64 {
    let front_sum: u32 = front.iter().map(|&n| n as u32).sum();
    let historical_sums: Vec<u32> = records.iter().map(|r| r.front.iter().map(|&n| n as u32).sum()).collect();
    let avg_sum = if historical_sums.is_empty() { 100.0 } else { historical_sums.iter().sum::<u32>() as f64 / historical_sums.len() as f64 };
    let sum_deviation = ((front_sum as f64 - avg_sum).abs() / avg_sum.max(1.0)).clamp(0.0, 1.0);
    let sum_score = 1.0 - sum_deviation;

    let spread = front.last().copied().unwrap_or(0) as f64 - front.first().copied().unwrap_or(0) as f64;
    let avg_spread = if records.is_empty() { 20.0 } else {
        records.iter().map(|r| r.front.last().copied().unwrap_or(0) as f64 - r.front.first().copied().unwrap_or(0) as f64)
            .sum::<f64>() / records.len() as f64
    };
    let spread_score = 1.0 - ((spread - avg_spread).abs() / avg_spread.max(1.0)).clamp(0.0, 1.0);

    let mut consecutive_count = 0usize;
    for window in front.windows(2) {
        if window[1] == window[0] + 1 { consecutive_count += 1; }
    }
    let consecutive_score = if consecutive_count >= 3 { 0.3 } else if consecutive_count == 2 { 0.7 } else { 1.0 };

    let mut quality_sum = 0.0;
    for &n in front {
        if let Some(stat) = front_stats.iter().find(|s| s.number == n) {
            let quality = 0.3 + stat.frequency * 0.3 + (1.0 - (stat.last_gap as f64 / 20.0).min(1.0)) * 0.4;
            quality_sum += quality;
        }
    }
    let quality_score = quality_sum / front.len().max(1) as f64;

    // Historical exact match penalty (already filtered, but double-check)
    let is_dup = records.iter().any(|r| {
        r.front.len() == front.len() && r.front.iter().all(|n| front.contains(n))
            && r.back.len() == back.len() && r.back.iter().all(|n| back.contains(n))
    });
    let novelty = if is_dup { 0.0 } else { 1.0 };

    sum_score * 0.20 + spread_score * 0.15 + consecutive_score * 0.20 + novelty * 0.25 + quality_score * 0.20
}

// ---- Number Confidence ----
fn compute_number_confidence_advanced(n: u8, stats: &[NumberStat], total_records: usize) -> f64 {
    if let Some(stat) = stats.iter().find(|s| s.number == n) {
        let total = total_records.max(1) as f64;
        let gap_factor = 1.0 - (stat.last_gap as f64 / total).clamp(0.0, 1.0) * 0.5;
        let freq_factor = (0.3 + stat.frequency * 0.5).clamp(0.0, 1.0);
        let physics_factor = (stat.physics_weight / 2.0).clamp(0.1, 0.9);
        (gap_factor * 0.35 + freq_factor * 0.35 + physics_factor * 0.30).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

// ---- Zone Targets (usize version) ----
fn compute_zone_targets_usize(records: &[DrawRecord], front_range: u8, pick_count: usize) -> Vec<usize> {
    let third = (front_range as f64 / 3.0).ceil() as u8;
    let mut zone_counts = [0usize; 3];
    let mut total = 0usize;
    for r in records.iter().take(10.min(records.len())) {
        for &n in &r.front {
            let idx = ((n - 1) / third).min(2) as usize;
            zone_counts[idx] += 1;
            total += 1;
        }
    }
    if total == 0 {
        return vec![pick_count / 3 + 1, pick_count / 3, pick_count / 3];
    }
    let avg: Vec<f64> = zone_counts.iter().map(|&c| c as f64 / total as f64 * pick_count as f64).collect();
    let mut rounded: Vec<usize> = avg.iter().map(|&v| v.round() as usize).collect();
    let sum: usize = rounded.iter().sum();
    if sum < pick_count {
        for i in 0..(pick_count - sum) { rounded[i % 3] += 1; }
    } else if sum > pick_count {
        for _i in 0..(sum - pick_count) {
            if let Some(max_idx) = rounded.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i) {
                if rounded[max_idx] > 0 { rounded[max_idx] -= 1; }
            }
        }
    }
    rounded
}

// ---- Odd/Even Target ----
fn compute_odd_even_target(records: &[DrawRecord], pick_count: usize) -> (usize, usize) {
    if records.is_empty() { return ((pick_count + 1) / 2, pick_count / 2); }
    let mut odd_total = 0usize;
    let mut even_total = 0usize;
    for record in records.iter().take(8.min(records.len())) {
        for &n in &record.front {
            if n % 2 == 1 { odd_total += 1; } else { even_total += 1; }
        }
    }
    let total = odd_total + even_total;
    if total == 0 { return ((pick_count + 1) / 2, pick_count / 2); }
    let odd_ratio = odd_total as f64 / total as f64;
    let odd_target = (odd_ratio * pick_count as f64).round() as usize;
    (odd_target.clamp(1, pick_count - 1), (pick_count - odd_target).clamp(1, pick_count - 1))
}

// ---- Supporting structs ----
#[derive(Debug, Clone)]
struct NumberStat {
    number: u8,
    frequency: f64,
    last_gap: usize,
    avg_gap: f64,
    streak_score: f64,
    zone: u8,
    is_odd: bool,
    physics_weight: f64,
}


fn compute_trend_alignment(front: &[u8], back: &[u8], records: &[DrawRecord]) -> f64 {
    if records.is_empty() { return 0.5; }
    let recent: Vec<&DrawRecord> = records.iter().take(5).collect();
    let mut match_count = 0usize;
    let mut total = 0usize;
    for record in &recent {
        for &n in front {
            if record.front.contains(&n) { match_count += 1; }
            total += 1;
        }
        for &n in back {
            if record.back.contains(&n) { match_count += 1; }
            total += 1;
        }
    }
    if total == 0 { return 0.5; }
    let ratio = match_count as f64 / total as f64;
    // Higher alignment with recent trends = higher score, but not TOO high (avoid exact repeats)
    (ratio * 1.5).clamp(0.1, 0.85)
}

fn evaluate_pattern_fit(front: &[u8], _back: &[u8], records: &[DrawRecord], front_range: u8) -> f64 {
    if records.len() < 2 { return 0.5; }

    // Zone distribution check
    let third = (front_range as f64 / 3.0).ceil() as u8;
    let mut zones = [0usize; 3];
    for &n in front {
        zones[((n - 1) / third).min(2) as usize] += 1;
    }
    let zone_uniformity = 1.0 - (zones.iter().max().copied().unwrap_or(0) as f64 - zones.iter().min().copied().unwrap_or(0) as f64) / front.len().max(1) as f64;

    // Odd/even check
    let odd_count = front.iter().filter(|&&n| n % 2 == 1).count();
    let even_count = front.len() - odd_count;
    let parity_balance = 1.0 - ((odd_count as f64 - even_count as f64).abs() / front.len().max(1) as f64);

    // Consecutive check
    let mut consecutive = 0usize;
    for w in front.windows(2) {
        if w[1] == w[0] + 1 { consecutive += 1; }
    }
    let consecutive_penalty = if consecutive > 2 { 0.4 } else if consecutive == 2 { 0.8 } else { 1.0 };

    // Historical pattern similarity
    let mut pattern_similarity = 0.0;
    for record in records.iter().take(8) {
        let front_overlap = front.iter().filter(|&&n| record.front.contains(&n)).count() as f64;
        pattern_similarity += front_overlap / front.len().max(1) as f64;
    }
    pattern_similarity = (pattern_similarity / 8.0).clamp(0.0, 1.0);

    zone_uniformity * 0.25 + parity_balance * 0.25 + consecutive_penalty * 0.25 + pattern_similarity * 0.25
}

fn evaluate_physics_fit(front: &[u8], back: &[u8], analysis: &AnalysisSummary) -> f64 {
    let front_profiles: Vec<&BallProfile> = analysis.ball_profiles.iter()
        .filter(|b| b.color == "red" && front.contains(&b.number)).collect();
    let back_profiles: Vec<&BallProfile> = analysis.ball_profiles.iter()
        .filter(|b| b.color != "red" && back.contains(&b.number)).collect();

    if front_profiles.is_empty() && back_profiles.is_empty() { return 0.5; }

    let mut total_score = 0.0;
    let mut count = 0usize;

    for p in &front_profiles {
        // Lighter, less friction balls are more likely to be drawn in physics simulation
        let physics_score = (0.5 + p.mass * 0.3 - p.friction * 0.4 + p.roughness * 0.2).clamp(0.1, 1.0);
        total_score += physics_score;
        count += 1;
    }
    for p in &back_profiles {
        let physics_score = (0.5 + p.mass * 0.3 - p.friction * 0.4 + p.roughness * 0.2).clamp(0.1, 1.0);
        total_score += physics_score;
        count += 1;
    }

    (total_score / count.max(1) as f64).clamp(0.0, 1.0)
}

#[tauri::command]
fn get_history() -> Vec<DrawRecord> {
    history()
}

#[tauri::command]
fn analyze_lottery(lottery: LotteryKind, replace_set_id: u32) -> AnalysisSummary {
    build_analysis(lottery, replace_set_id)
}

#[tauri::command]
fn simulate_draw(req: DrawRequest) -> DrawResult {
    let analysis = build_analysis(req.lottery, req.replace_set_id);
    let seed = req.seed.unwrap_or_else(|| {
        let issue_hash = req
            .target_issue
            .as_deref()
            .unwrap_or("live")
            .bytes()
            .fold(17_u64, |acc, b| acc.wrapping_mul(131).wrapping_add(b as u64));
        issue_hash ^ ((req.replace_set_id as u64) << 32)
    });
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut explanation = vec![
        "根据历史频率、冷热号、分区与奇偶分布生成每个球的初始权重。".to_string(),
        "球组编号会改变质量、摩擦系数和表面不规则度，用来模拟一键换球后的微小差异。".to_string(),
    ];

    let (front_pick, back_pick) = (analysis.front_pick as usize, analysis.back_pick as usize);
    let mut front_pool: Vec<&BallProfile> = analysis.ball_profiles.iter().filter(|b| b.color == "red").collect();
    let mut back_pool: Vec<&BallProfile> = analysis.ball_profiles.iter().filter(|b| b.color != "red").collect();

    let front = pick_without_replacement(&mut rng, &mut front_pool, front_pick, req.mode == "calibrated");
    let back = pick_without_replacement(&mut rng, &mut back_pool, back_pick, req.mode == "calibrated");

    let target = req
        .target_issue
        .as_ref()
        .and_then(|issue| history().into_iter().find(|r| r.lottery == req.lottery && r.issue == *issue));
    let exact_history_match = target
        .as_ref()
        .map(|record| record.front == front && record.back == back)
        .unwrap_or(false);

    if exact_history_match {
        explanation.push("本轮模拟号码与选定历史期号完全一致，可作为当前参数组的一次通过样本。".to_string());
    } else if target.is_some() {
        explanation.push("本轮未完全复现目标历史期号；可换球或调整校准模式继续测试参数稳定性。".to_string());
    }

    DrawResult {
        request_id: format!("SIM-{}-{}", req.replace_set_id, seed % 100_000),
        lottery: req.lottery,
        issue: target.map(|r| r.issue).unwrap_or_else(|| "模拟即时开奖".to_string()),
        station: station_for(&mut rng),
        front,
        back,
        spin_seconds: rng.random_range(12.5..24.0),
        chamber_rpm: rng.random_range(58.0..86.0),
        collision_count: rng.random_range(900..2600),
        energy_loss: rng.random_range(0.18..0.42),
        exact_history_match,
        explanation,
    }
}

#[tauri::command]
fn run_backtest(lottery: LotteryKind, replace_set_id: u32, attempts_per_issue: u32) -> BacktestResult {
    let records: Vec<_> = history().into_iter().filter(|r| r.lottery == lottery).collect();
    let attempts = attempts_per_issue.clamp(1, 80);
    let mut exact_matches = 0;
    let mut front_hits_total = 0.0;
    let mut back_hits_total = 0.0;
    let mut best_issue = String::new();
    let mut best_score = -1.0;

    for record in &records {
        for attempt in 0..attempts {
            let req = DrawRequest {
                lottery,
                mode: "calibrated".to_string(),
                seed: Some(seed_from_issue(&record.issue, replace_set_id, attempt)),
                replace_set_id,
                target_issue: Some(record.issue.clone()),
            };
            let result = simulate_draw(req);
            let front_hits = result.front.iter().filter(|n| record.front.contains(n)).count() as f64;
            let back_hits = result.back.iter().filter(|n| record.back.contains(n)).count() as f64;
            let score = front_hits + back_hits * 1.7;
            front_hits_total += front_hits;
            back_hits_total += back_hits;
            if result.exact_history_match {
                exact_matches += 1;
            }
            if score > best_score {
                best_score = score;
                best_issue = record.issue.clone();
            }
        }
    }

    let denominator = (records.len() as u32 * attempts) as f64;
    BacktestResult {
        lottery,
        sample_count: records.len() as u32 * attempts,
        exact_matches,
        average_front_hits: round2(front_hits_total / denominator),
        average_back_hits: round2(back_hits_total / denominator),
        best_issue,
        best_score: round2(best_score),
        notes: vec![
            "彩票开奖本质是随机事件，历史拟合只能验证模拟器参数与样本分布的贴近程度，不能用于预测真实结果。".to_string(),
            "exactMatches 表示模拟过程完全复现历史开奖号码的次数；命中均值更适合观察参数组是否偏离样本分布。".to_string(),
        ],
    }
}

fn build_analysis(lottery: LotteryKind, replace_set_id: u32) -> AnalysisSummary {
    let (front_range, back_range, front_pick, back_pick) = match lottery {
        LotteryKind::Ssq => (33, 16, 6, 1),
        LotteryKind::Dlt => (35, 12, 5, 2),
    };
    let records: Vec<_> = history().into_iter().filter(|r| r.lottery == lottery).collect();
    let mut front_counts = count_numbers(&records, true, front_range);
    let mut back_counts = count_numbers(&records, false, back_range);

    let hot_front = top_numbers(&front_counts, true, 6);
    let cold_front = top_numbers(&front_counts, false, 6);
    let hot_back = top_numbers(&back_counts, true, 4);
    let cold_back = top_numbers(&back_counts, false, 4);
    let zone_balance = zones(&records, front_range);
    let odd_even_balance = odd_even(&records);

    let mut ball_profiles = Vec::new();
    for number in 1..=front_range {
        ball_profiles.push(profile_for(number, "red", front_counts.remove(&number).unwrap_or(0), replace_set_id));
    }
    for number in 1..=back_range {
        let color = match lottery {
            LotteryKind::Ssq => "blue",
            LotteryKind::Dlt => "blue",
        };
        ball_profiles.push(profile_for(number, color, back_counts.remove(&number).unwrap_or(0), replace_set_id + 97));
    }

    AnalysisSummary {
        lottery,
        front_range,
        back_range,
        front_pick,
        back_pick,
        hot_front,
        cold_front,
        hot_back,
        cold_back,
        zone_balance,
        odd_even_balance,
        ball_profiles,
    }
}

fn pick_without_replacement(
    rng: &mut ChaCha8Rng,
    pool: &mut Vec<&BallProfile>,
    count: usize,
    calibrated: bool,
) -> Vec<u8> {
    let mut picked = Vec::new();
    for _ in 0..count {
        let total: f64 = pool
            .iter()
            .map(|b| {
                let physical = 1.0 + (b.roughness - 0.5) * 0.35 - b.friction * 0.22 + (b.mass - 1.0) * 0.18;
                if calibrated { b.bias * physical } else { physical.max(0.15) }
            })
            .sum();
        let mut cursor = rng.random_range(0.0..total.max(0.1));
        let mut index = 0;
        for (i, ball) in pool.iter().enumerate() {
            let weight = if calibrated {
                ball.bias * (1.0 + ball.heat * 0.12)
            } else {
                1.0 + ball.roughness * 0.1
            };
            cursor -= weight.max(0.05);
            if cursor <= 0.0 {
                index = i;
                break;
            }
        }
        picked.push(pool.remove(index).number);
    }
    picked.sort_unstable();
    picked
}

fn profile_for(number: u8, color: &str, hits: u32, set_id: u32) -> BallProfile {
    let x = ((number as u32 * 37 + set_id * 19) % 100) as f64 / 100.0;
    let y = ((number as u32 * 73 + set_id * 11) % 100) as f64 / 100.0;
    let heat = hits as f64 / 10.0;
    BallProfile {
        number,
        color: color.to_string(),
        mass: round3(0.94 + x * 0.12),
        friction: round3(0.18 + y * 0.28),
        roughness: round3(0.12 + ((x + y) / 2.0) * 0.72),
        bias: round3(0.8 + heat * 0.22 + x * 0.12),
        heat: round3(heat),
    }
}

fn count_numbers(records: &[DrawRecord], front: bool, range: u8) -> HashMap<u8, u32> {
    let mut map = (1..=range).map(|n| (n, 0)).collect::<HashMap<_, _>>();
    for record in records {
        let nums = if front { &record.front } else { &record.back };
        for number in nums {
            *map.entry(*number).or_insert(0) += 1;
        }
    }
    map
}

fn top_numbers(counts: &HashMap<u8, u32>, desc: bool, limit: usize) -> Vec<u8> {
    let mut values: Vec<(u8, u32)> = counts.iter().map(|(n, c)| (*n, *c)).collect();
    values.sort_by(|a, b| if desc { b.1.cmp(&a.1).then(a.0.cmp(&b.0)) } else { a.1.cmp(&b.1).then(a.0.cmp(&b.0)) });
    values.into_iter().take(limit).map(|(n, _)| n).collect()
}

fn zones(records: &[DrawRecord], range: u8) -> Vec<u32> {
    let third = (range as f64 / 3.0).ceil() as u8;
    let mut zones = vec![0, 0, 0];
    for record in records {
        for number in &record.front {
            let idx = ((*number - 1) / third).min(2) as usize;
            zones[idx] += 1;
        }
    }
    zones
}

fn odd_even(records: &[DrawRecord]) -> Vec<u32> {
    let mut odd = 0;
    let mut even = 0;
    for record in records {
        for number in record.front.iter().chain(record.back.iter()) {
            if number % 2 == 0 { even += 1 } else { odd += 1 }
        }
    }
    vec![odd, even]
}

fn seed_from_issue(issue: &str, set_id: u32, attempt: u32) -> u64 {
    issue.bytes().fold(991_u64, |acc, b| acc.wrapping_mul(167).wrapping_add(b as u64))
        ^ ((set_id as u64) << 24)
        ^ attempt as u64
}

fn station_for(rng: &mut ChaCha8Rng) -> String {
    let stations = [
        "北京丰台开奖中心",
        "上海浦东数据校验站",
        "广东广州省级开奖站",
        "四川成都公益彩票中心",
        "浙江杭州复核站",
    ];
    stations[rng.random_range(0..stations.len())].to_string()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn history() -> Vec<DrawRecord> {
    vec![
        ssq("SSQ-2026060", "2026060", "2026-05-28", vec![3, 7, 12, 18, 24, 31], vec![9], 7, 418.2, "北京丰台开奖中心"),
        ssq("SSQ-2026059", "2026059", "2026-05-26", vec![1, 9, 14, 19, 27, 33], vec![5], 10, 436.7, "广东广州省级开奖站"),
        ssq("SSQ-2026058", "2026058", "2026-05-24", vec![5, 11, 16, 20, 22, 30], vec![12], 6, 401.8, "上海浦东数据校验站"),
        ssq("SSQ-2026057", "2026057", "2026-05-21", vec![2, 6, 15, 23, 26, 32], vec![3], 8, 429.3, "浙江杭州复核站"),
        ssq("SSQ-2026056", "2026056", "2026-05-19", vec![4, 8, 13, 17, 25, 29], vec![15], 4, 397.6, "四川成都公益彩票中心"),
        ssq("SSQ-2026055", "2026055", "2026-05-17", vec![6, 10, 12, 21, 28, 31], vec![8], 12, 451.1, "北京丰台开奖中心"),
        ssq("SSQ-2026054", "2026054", "2026-05-14", vec![7, 9, 18, 20, 24, 33], vec![2], 5, 390.9, "广东广州省级开奖站"),
        ssq("SSQ-2026053", "2026053", "2026-05-12", vec![1, 5, 11, 16, 27, 30], vec![11], 9, 432.5, "上海浦东数据校验站"),
        dlt("DLT-2026060", "2026060", "2026-05-28", vec![4, 9, 16, 22, 34], vec![3, 10], 3, 318.4, "天津体彩开奖中心"),
        dlt("DLT-2026059", "2026059", "2026-05-26", vec![2, 7, 15, 25, 31], vec![5, 12], 4, 330.2, "广东体彩复核站"),
        dlt("DLT-2026058", "2026058", "2026-05-24", vec![6, 11, 18, 23, 29], vec![2, 8], 2, 304.6, "浙江体彩开奖站"),
        dlt("DLT-2026057", "2026057", "2026-05-21", vec![1, 13, 17, 21, 35], vec![1, 9], 5, 349.0, "四川体彩开奖站"),
        dlt("DLT-2026056", "2026056", "2026-05-19", vec![3, 8, 14, 20, 28], vec![4, 11], 2, 298.7, "天津体彩开奖中心"),
        dlt("DLT-2026055", "2026055", "2026-05-17", vec![5, 12, 19, 26, 33], vec![6, 7], 6, 362.9, "广东体彩复核站"),
        dlt("DLT-2026054", "2026054", "2026-05-14", vec![9, 10, 16, 24, 30], vec![2, 12], 1, 287.5, "浙江体彩开奖站"),
        dlt("DLT-2026053", "2026053", "2026-05-12", vec![7, 11, 15, 27, 32], vec![3, 8], 4, 321.6, "四川体彩开奖站"),
    ]
}

fn ssq(id: &str, issue: &str, date: &str, front: Vec<u8>, back: Vec<u8>, jackpot_count: u32, sales_million: f64, station: &str) -> DrawRecord {
    DrawRecord { id: id.into(), issue: issue.into(), date: date.into(), lottery: LotteryKind::Ssq, station: station.into(), sales_million, jackpot_count, front, back }
}

fn dlt(id: &str, issue: &str, date: &str, front: Vec<u8>, back: Vec<u8>, jackpot_count: u32, sales_million: f64, station: &str) -> DrawRecord {
    DrawRecord { id: id.into(), issue: issue.into(), date: date.into(), lottery: LotteryKind::Dlt, station: station.into(), sales_million, jackpot_count, front, back }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_history,
            analyze_lottery,
            simulate_draw,
            run_backtest,
            predict_draw
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
