use std::collections::{HashMap, HashSet};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// ---- Advanced Prediction Engine ----

#[derive(Debug, Clone)]
struct NumberStats {
    number: u8,
    frequency: f64,
    last_gap: usize,
    avg_gap: f64,
    streak_score: f64,
    zone: u8,
    is_odd: bool,
    physics_weight: f64,
}

fn build_number_stats(
    records: &[DrawRecord],
    pool: &[&BallProfile],
    range: u8,
    is_front: bool,
) -> Vec<NumberStats> {
    let mut stats = Vec::new();
    let total_records = records.len().max(1) as f64;

    for &profile in pool {
        let n = profile.number;
        if n > range { continue; }

        // Frequency with exponential recency weighting
        let mut weighted_freq = 0.0;
        let mut last_seen: Option<usize> = None;
        let mut gaps = Vec::new();
        let mut prev_idx: Option<usize> = None;

        for (idx, record) in records.iter().enumerate() {
            let numbers = if is_front { &record.front } else { &record.back };
            let decay = (-0.15 * idx as f64).exp();
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

        // Streak score: numbers that appear in clusters get bonus
        let streak_score = if gaps.len() >= 2 {
            let recent_gaps: Vec<usize> = gaps.iter().rev().take(3).copied().collect();
            let avg_recent = recent_gaps.iter().sum::<usize>() as f64 / recent_gaps.len() as f64;
            if avg_recent < 2.5 { 1.3 } else if avg_recent < 4.0 { 1.1 } else { 0.9 }
        } else {
            1.0
        };

        let third = (range as f64 / 3.0).ceil() as u8;
        let zone = ((n - 1) / third).min(2);

        // Physics weight: lighter, smoother balls more likely to be drawn
        let physics_weight = (0.6 + profile.mass * 0.3 - profile.friction * 0.4 + profile.roughness * 0.2)
            .clamp(0.2, 1.8);

        stats.push(NumberStats {
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

fn compute_composite_score(stat: &NumberStats, total_records: usize, hot_numbers: &HashSet<u8>, cold_numbers: &HashSet<u8>) -> f64 {
    let total = total_records.max(1) as f64;

    // Gap score: overdue numbers (high gap) get higher probability
    let gap_ratio = stat.last_gap as f64 / total;
    let gap_score = if gap_ratio > 0.7 { 1.4 } else if gap_ratio > 0.5 { 1.2 } else if gap_ratio > 0.3 { 1.0 } else { 0.8 };

    // Frequency score
    let freq_score = 0.5 + stat.frequency * 0.8;

    // Hot/cold bonus
    let hot_cold_bonus = if hot_numbers.contains(&stat.number) {
        1.15
    } else if cold_numbers.contains(&stat.number) {
        1.25 // Cold numbers are "due"
    } else {
        1.0
    };

    // Physics
    let physics_score = stat.physics_weight;

    // Streak
    let streak = stat.streak_score;

    let raw = gap_score * 0.25 + freq_score * 0.20 + hot_cold_bonus * 0.20 + physics_score * 0.20 + streak * 0.15;
    raw.clamp(0.1, 3.0)
}

fn pick_numbers_with_constraints(
    rng: &mut ChaCha8Rng,
    stats: &[NumberStats],
    pick_count: usize,
    zone_targets: &[usize],
    odd_even_target: (usize, usize),
    hot_numbers: &HashSet<u8>,
    cold_numbers: &HashSet<u8>,
    total_records: usize,
) -> Vec<u8> {
    let mut available: Vec<&NumberStats> = stats.iter().collect();
    let mut picked: Vec<u8> = Vec::new();
    let mut zone_counts = [0usize; 3];
    let mut odd_count = 0usize;
    let mut even_count = 0usize;

    for i in 0..pick_count {
        let remaining = pick_count - i;

        let weights: Vec<f64> = available.iter().map(|&s| {
            let base = compute_composite_score(s, total_records, hot_numbers, cold_numbers);

            // Zone constraint: boost underrepresented zones
            let zone_boost = if zone_counts[s.zone as usize] < zone_targets[s.zone as usize] {
                1.5
            } else if zone_counts[s.zone as usize] >= zone_targets[s.zone as usize] + 1 {
                0.3
            } else {
                1.0
            };

            // Odd/even constraint
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

            // Ensure we leave enough variety for remaining picks
            let diversity_boost = if remaining <= 2 && zone_counts.iter().any(|&c| c == 0) {
                if zone_counts[s.zone as usize] == 0 { 2.0 } else { 0.5 }
            } else {
                1.0
            };

            base * zone_boost * parity_boost * diversity_boost
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

fn is_historical_match(front: &[u8], back: &[u8], records: &[DrawRecord]) -> bool {
    for record in records {
        let front_match = record.front.len() == front.len() && record.front.iter().all(|n| front.contains(n));
        let back_match = record.back.len() == back.len() && record.back.iter().all(|n| back.contains(n));
        if front_match && back_match {
            return true;
        }
    }
    false
}

fn compute_zone_targets_from_history(records: &[DrawRecord], front_range: u8, pick_count: usize) -> Vec<usize> {
    let third = (front_range as f64 / 3.0).ceil() as u8;
    let mut zone_counts = [0usize; 3];
    let mut total = 0usize;

    for record in records.iter().take(10.min(records.len())) {
        for &n in &record.front {
            let idx = ((n - 1) / third).min(2) as usize;
            zone_counts[idx] += 1;
            total += 1;
        }
    }

    if total == 0 {
        return vec![pick_count / 3 + 1, pick_count / 3, pick_count / 3];
    }

    let avg: Vec<f64> = zone_counts.iter().map(|&c| c as f64 / total as f64 * pick_count as f64).collect();
    let rounded: Vec<usize> = avg.iter().map(|&v| v.round() as usize).collect();
    let sum: usize = rounded.iter().sum();

    // Adjust to match pick_count
    let mut adjusted = rounded.clone();
    if sum < pick_count {
        for i in 0..(pick_count - sum) {
            adjusted[i % 3] += 1;
        }
    } else if sum > pick_count {
        for i in 0..(sum - pick_count) {
            if let Some(min_idx) = adjusted.iter().enumerate().max_by_key(|(_, v)| **v).map(|(i, _)| i) {
                if adjusted[min_idx] > 0 { adjusted[min_idx] -= 1; }
            }
        }
    }
    adjusted
}

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

fn evaluate_candidate_advanced(
    front: &[u8], back: &[u8],
    stats_front: &[NumberStats],
    stats_back: &[NumberStats],
    records: &[DrawRecord],
) -> f64 {
    // Sum analysis
    let front_sum: u32 = front.iter().map(|&n| n as u32).sum();
    let historical_sums: Vec<u32> = records.iter().map(|r| r.front.iter().map(|&n| n as u32).sum()).collect();
    let avg_sum = if historical_sums.is_empty() { 100.0 } else { historical_sums.iter().sum::<u32>() as f64 / historical_sums.len() as f64 };
    let sum_deviation = ((front_sum as f64 - avg_sum).abs() / avg_sum.max(1.0)).clamp(0.0, 1.0);
    let sum_score = 1.0 - sum_deviation;

    // Spread analysis (range between min and max)
    let spread = front.last().copied().unwrap_or(0) as f64 - front.first().copied().unwrap_or(0) as f64;
    let historical_spreads: Vec<f64> = records.iter()
        .map(|r| r.back.first().map(|_| 0.0).unwrap_or(0.0) + r.front.last().copied().unwrap_or(0) as f64 - r.front.first().copied().unwrap_or(0) as f64)
        .collect();
    let avg_spread = if historical_spreads.is_empty() { 20.0 } else { historical_spreads.iter().sum::<f64>() / historical_spreads.len() as f64 };
    let spread_score = 1.0 - ((spread - avg_spread).abs() / avg_spread.max(1.0)).clamp(0.0, 1.0);

    // Consecutive numbers penalty (too many consecutive = unnatural)
    let mut consecutive_count = 0usize;
    for window in front.windows(2) {
        if window[1] == window[0] + 1 { consecutive_count += 1; }
    }
    let consecutive_score = if consecutive_count >= 3 { 0.3 } else if consecutive_count == 2 { 0.7 } else { 1.0 };

    // No historical match bonus
    let novelty_bonus = if is_historical_match(front, back, records) { 0.0 } else { 1.0 };

    // Number quality score
    let mut quality_sum = 0.0;
    for &n in front {
        if let Some(stat) = stats_front.iter().find(|s| s.number == n) {
            let quality = 0.3 + stat.frequency * 0.3 + (1.0 - (stat.last_gap as f64 / 20.0).min(1.0)) * 0.4;
            quality_sum += quality;
        }
    }
    let quality_score = quality_sum / front.len().max(1) as f64;

    sum_score * 0.20 + spread_score * 0.15 + consecutive_score * 0.20 + novelty_bonus * 0.25 + quality_score * 0.20
}
