/*
 * Copyright (c) 2022, Andreas Kling <kling@serenityos.org>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use serenity::{json, sys};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    sys::pledge("stdio rpath")?;
    sys::unveil("/proc/interrupts", "r")?;
    sys::lock_veil()?;
    let json = json::parse(std::fs::read_to_string("/proc/interrupts")?.as_str())?;
    sys::pledge("stdio")?;
    println!("      CPU0");
    if let json::Value::Array(array) = &json {
        for value in array {
            if let json::Value::Object(handler) = value {
                let purpose = handler.get("purpose").unwrap();
                let interrupt_line = handler.get("interrupt_line").unwrap();
                let controller = handler.get("controller").unwrap();
                let call_count = handler.get("call_count").unwrap();

                println!(
                    "{:>4}: {:10} {:10}  {:30}",
                    interrupt_line.to_string(),
                    call_count.to_string(),
                    controller.to_string(),
                    purpose.to_string()
                );
            }
        }
    }
    Ok(())
}
