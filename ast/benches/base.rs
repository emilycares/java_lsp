use ast::lexer;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let content = include_str!("../../parser/test/Everything.java");
    c.bench_function("lexer Everything.java", |b| {
        b.iter(|| lexer::lex(black_box(content)))
    });
    let tokens = lexer::lex(content).unwrap();
    c.bench_function("ast Everything.java", |b| {
        b.iter(|| ast::parse_file(black_box(&tokens)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
