use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hl_md_handler::engine::{Engine, MULTIPLIER};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::hint::black_box;
use std::str::FromStr;

fn parse_str_to_u64(c: &mut Criterion) {
    let inputs = ["123.456", "123", "123456.7"];
    let mut group = c.benchmark_group("parse_str_to_u64");

    for s in inputs {
        group.bench_with_input(BenchmarkId::new("engine_parse", s), s, |b, s| {
            b.iter(|| black_box(Engine::parse_to_u64_with_mul(black_box(s))))
        });

        group.bench_with_input(BenchmarkId::new("f64_parse", s), s, |b, s| {
            b.iter(|| {
                black_box((black_box(s).parse::<f64>().unwrap() * MULTIPLIER as f64).round() as u64)
            })
        });

        group.bench_with_input(BenchmarkId::new("decimal_parse", s), s, |b, s| {
            b.iter(|| {
                black_box(
                    (Decimal::from_str(black_box(s)).unwrap() * Decimal::from(MULTIPLIER))
                        .round()
                        .to_u64()
                        .unwrap(),
                )
            })
        });
    }

    group.finish();
}

criterion_group!(benches, parse_str_to_u64);
criterion_main!(benches);
