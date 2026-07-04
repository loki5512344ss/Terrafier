use std::collections::HashMap;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use terrafier_nbt::tag::Tag;
use terrafier_nbt::{reader, writer};

fn make_small_compound() -> Tag {
    let mut map = HashMap::new();
    map.insert("byte".into(), Tag::Byte(42));
    map.insert("short".into(), Tag::Short(1000));
    map.insert("int".into(), Tag::Int(1_000_000));
    map.insert("long".into(), Tag::Long(1_000_000_000));
    map.insert("float".into(), Tag::Float(std::f64::consts::PI as f32));
    map.insert("double".into(), Tag::Double(std::f64::consts::E));
    map.insert("string".into(), Tag::String("Hello, Benchmark!".into()));
    map.insert(
        "inner".into(),
        Tag::Compound(HashMap::from([
            ("x".into(), Tag::Int(10)),
            ("y".into(), Tag::Int(20)),
        ])),
    );
    map.insert("arr".into(), Tag::IntArray(vec![1, 2, 3, 4, 5]));
    map.insert("flag".into(), Tag::Byte(1));
    Tag::Compound(map)
}

fn make_large_long_array() -> Tag {
    let vals: Vec<i64> = (0..4096).map(|i| i as i64 * 31 + 7).collect();
    Tag::Compound(HashMap::from([("data".into(), Tag::LongArray(vals))]))
}

fn make_gzip_roundtrip_tag() -> Tag {
    Tag::Compound(HashMap::from([
        ("version".into(), Tag::Int(3)),
        (
            "data".into(),
            Tag::ByteArray((0..512).map(|i| i as i8).collect()),
        ),
    ]))
}

fn bench_nbt_parse_small(c: &mut Criterion) {
    let tag = make_small_compound();
    let bytes = writer::to_bytes(&tag).unwrap();

    c.bench_function("nbt/parse_small_compound", |b| {
        b.iter(|| {
            let result = reader::read_bytes(black_box(&bytes)).unwrap();
            black_box(result);
        });
    });
}

fn bench_nbt_parse_large_array(c: &mut Criterion) {
    let tag = make_large_long_array();
    let bytes = writer::to_bytes(&tag).unwrap();

    c.bench_function("nbt/parse_large_longarray_4096", |b| {
        b.iter(|| {
            let result = reader::read_bytes(black_box(&bytes)).unwrap();
            black_box(result);
        });
    });
}

fn bench_nbt_serialize_compound(c: &mut Criterion) {
    let tag = make_small_compound();

    c.bench_function("nbt/serialize_compound", |b| {
        b.iter(|| {
            let result = writer::to_bytes(black_box(&tag)).unwrap();
            black_box(result);
        });
    });
}

fn bench_nbt_serialize_large_array(c: &mut Criterion) {
    let tag = make_large_long_array();

    c.bench_function("nbt/serialize_large_longarray", |b| {
        b.iter(|| {
            let result = writer::to_bytes(black_box(&tag)).unwrap();
            black_box(result);
        });
    });
}

fn bench_nbt_gzip_roundtrip(c: &mut Criterion) {
    let tag = make_gzip_roundtrip_tag();

    c.bench_function("nbt/gzip_roundtrip", |b| {
        b.iter(|| {
            let compressed = writer::to_gzip_bytes(black_box(&tag)).unwrap();
            let _parsed = reader::read_gzip(&compressed).unwrap();
            black_box(compressed);
        });
    });
}

fn bench_nbt_roundtrip_small(c: &mut Criterion) {
    let tag = make_small_compound();

    c.bench_function("nbt/roundtrip_small", |b| {
        b.iter(|| {
            let bytes = writer::to_bytes(black_box(&tag)).unwrap();
            let _parsed = reader::read_bytes(&bytes).unwrap();
            black_box(bytes);
        });
    });
}

criterion_group!(
    nbt,
    bench_nbt_parse_small,
    bench_nbt_parse_large_array,
    bench_nbt_serialize_compound,
    bench_nbt_serialize_large_array,
    bench_nbt_gzip_roundtrip,
    bench_nbt_roundtrip_small
);
criterion_main!(nbt);
