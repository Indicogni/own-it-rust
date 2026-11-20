#![allow(warnings)]

use std::collections::HashMap;

pub struct Entry {
    pub date: String,
    pub category: String,
    pub cents: i64,
}

pub fn parse_line(line: &str) -> Entry {
    let date = &line[..10];
    let rest = &line[11..];
    let parts: Vec<&str> = rest.split(' ').collect();
    let category = parts[0];
    let amount = parts[1];
    let pieces: Vec<&str> = amount.split('.').collect();
    let cents = pieces[0].parse::<i64>().unwrap() * 100 + pieces[1].parse::<i64>().unwrap();
    return Entry { date: date.to_string(), category: category.to_string(), cents: cents };
}

pub fn parse(text: &str) -> Vec<Entry> {
    let mut v = Vec::new();
    for l in text.lines() {
        if l.len() == 0 { continue; }
        if l.starts_with("#") { continue; }
        v.push(parse_line(l));
    }
    v
}

pub fn totals(entries: &Vec<Entry>) -> HashMap<String, i64> {
    let mut m = HashMap::new();
    for i in 0..entries.len() {
        *m.entry(entries[i].category.clone()).or_insert(0) += entries[i].cents;
    }
    m
}

pub fn report(entries: &Vec<Entry>) -> String {
    let t = totals(entries);
    let mut s = String::new();
    let mut total = 0;
    for (k, v) in t.iter() {
        s = s + &format!("{}  {}.{:02}\n", k, v / 100, v % 100);
        total += v;
    }
    s = s + &format!("total  {}.{:02}\n", total / 100, total % 100);
    s
}

pub fn to_json(entries: &Vec<Entry>) -> String {
    let mut s = String::from("[");
    for (i, e) in entries.iter().enumerate() {
        if i > 0 { s.push(','); }
        s = s + &format!("{{\"date\":\"{}\",\"category\":\"{}\",\"cents\":{}}}", e.date, e.category, e.cents);
    }
    s.push(']');
    s
}
