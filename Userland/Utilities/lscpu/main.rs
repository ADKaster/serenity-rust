/*
 * Copyright (c) 2022, Linus Groh <linusg@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::collections::HashMap;

use serenity::{json, sys};

fn print_cpu_info(processor: &HashMap<String, json::Value>) {
    let processor_id = processor.get("processor").unwrap();
    let vendor_id = processor.get("vendor_id").unwrap();
    let brand = processor.get("brand").unwrap();
    let family = processor.get("family").unwrap();
    let model = processor.get("model").unwrap();
    let stepping = processor.get("stepping").unwrap();
    let r#type = processor.get("type").unwrap();
    let features = processor.get("features").unwrap();

    println!("CPU {}:", processor_id.to_string());
    println!("\tVendor ID: {}", vendor_id.to_string());
    if processor.contains_key("hypervisor_vendor_id") {
        let hypervisor_vendor_id = processor.get("hypervisor_vendor_id").unwrap();
        println!(
            "\tHypervisor Vendor ID: {}",
            hypervisor_vendor_id.to_string(),
        );
    }
    println!("\tBrand: {}", brand.to_string());
    println!("\tFamily: {}", family.to_string());
    println!("\tModel: {}", model.to_string());
    println!("\tStepping: {}", stepping.to_string());
    println!("\tType: {}", r#type.to_string());
    println!(
        "\tFeatures: {}",
        features
            .as_array()
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    sys::pledge("stdio rpath")?;
    sys::unveil("/proc/cpuinfo", "r")?;
    sys::lock_veil()?;
    let json = json::parse(std::fs::read_to_string("/proc/cpuinfo")?.as_str())?;
    sys::pledge("stdio")?;
    let processors = json.as_array();
    let mut it = processors.iter().peekable();
    while let Some(value) = it.next() {
        let processor = value.as_object();
        print_cpu_info(processor);
        if !it.peek().is_none() {
            println!();
        }
    }
    Ok(())
}
