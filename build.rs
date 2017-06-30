// Copyright (C) 2017 Stephane Raux. Distributed under the MIT license.

#![deny(warnings)]

extern crate bindgen;
extern crate cmake;
extern crate git2;

use git2::{Repository, SubmoduleIgnore};
use std::env;
use std::path::Path;

fn main() {
    let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = Path::new(&src_dir);
    {
        let repo = Repository::open(src_dir).unwrap();
        if repo.submodule_status("clue", SubmoduleIgnore::Unspecified).unwrap()
                .contains(git2::SUBMODULE_STATUS_WD_UNINITIALIZED) {
            println!("Updating submodules");
            update_submodules(repo).unwrap();
        }
    }
    let lib_dir = cmake::Config::new(&src_dir.join("clue")).build();
    let lib_dir = lib_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=clue");
    let target = env::var("TARGET").unwrap();
    if target.contains("-darwin-") {
        println!("cargo:rustc-link-lib=c++");
    } else if !target.contains("-win32-") {
        println!("cargo:rustc-link-lib=stdc++");
    }
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    println!("cargo:root={}", out_dir.display());
    let bindings = bindgen::Builder::default()
        .unstable_rust(false)
        .clang_arg(format!("-I{}", out_dir.join("include").display()))
        .header(out_dir.join("include/clue/clue.h").to_str().unwrap())
        // Bindgen generated unit tests fail because of this type.
        // https://github.com/servo/rust-bindgen/issues/550
        .hide_type("max_align_t")
        .generate()
        .unwrap();
    bindings.write_to_file(out_dir.join("bindings.rs")).unwrap();
}

fn update_submodules(repo: Repository) -> Result<(), git2::Error> {
    let mut repos = vec![repo];
    while let Some(repo) = repos.pop() {
        for mut subm in repo.submodules()? {
            subm.update(true, None)?;
            repos.push(subm.open()?);
        }
    }
    Ok(())
}
