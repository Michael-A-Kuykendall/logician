// # Spell: FullIntegration
// Benchmark for SMT operations using criterion

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use logician::driver::Config;
use logician::parser::parse;
use logician::term::{Term, Sort, And2, Or2};
use smallvec::smallvec;
use std::time::Duration;

fn bench_term_construction(c: &mut Criterion) {
    c.bench_function("term_and_chain", |b| {
        b.iter(|| {
            let t1 = Term::Bool(true);
            let t2 = Term::Bool(false);
            let t3 = Term::Bool(true);
            black_box(t1.and(t2).and(t3))
        })
    });
    
    c.bench_function("term_or_chain", |b| {
        b.iter(|| {
            let t1 = Term::Bool(true);
            let t2 = Term::Bool(false);
            let t3 = Term::Bool(true);
            black_box(t1.or(t2).or(t3))
        })
    });
    
    c.bench_function("term_nested_not", |b| {
        b.iter(|| {
            let t = Term::Bool(true);
            black_box(t.not().not().not())
        })
    });
}

fn bench_serialization(c: &mut Criterion) {
    c.bench_function("serialize_complex_term", |b| {
        let complex = Term::And(
            And2(
                Box::new(Term::Var("x".into(), Sort::Bool)),
                Box::new(Term::Or(
                    Or2(
                        Box::new(Term::Var("y".into(), Sort::Bool)),
                        Box::new(Term::Not(Box::new(Term::Var("z".into(), Sort::Bool)))),
                    ),
                    smallvec![],
                )),
            ),
            smallvec![Box::new(Term::Bool(true))],
        );
        
        b.iter(|| {
            black_box(format!("{}", complex))
        })
    });
}

fn bench_parsing(c: &mut Criterion) {
    c.bench_function("parse_sat", |b| {
        b.iter(|| {
            black_box(parse("sat"))
        })
    });
    
    c.bench_function("parse_model", |b| {
        let model = r#"(define-fun x () Int 42)
(define-fun y () Bool true)"#;
        b.iter(|| {
            black_box(parse(model))
        })
    });
}

fn bench_config_creation(c: &mut Criterion) {
    c.bench_function("config_create", |b| {
        b.iter(|| {
            black_box(Config {
                program: "z3".into(),
                args: vec!["-in".into()],
                timeout: Duration::from_secs(30),
                trace: false,
            })
        })
    });
}

criterion_group!(
    benches,
    bench_term_construction,
    bench_serialization,
    bench_parsing,
    bench_config_creation,
);
criterion_main!(benches);
