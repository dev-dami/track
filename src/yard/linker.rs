use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LinkerEngine;

impl LinkerEngine {
    /// Detect if fast linkers like `mold` or `lld` are available.
    pub fn detect_fast_linker_flag() -> Option<&'static str> {
        if Command::new("mold").arg("--version").output().is_ok() {
            Some("-fuse-ld=mold")
        } else if Command::new("ld.lld").arg("--version").output().is_ok() {
            Some("-fuse-ld=lld")
        } else {
            None
        }
    }

    /// Compile and cache the C runtime helper object file into target directory.
    pub fn get_or_compile_runtime(target_dir: &Path) -> Result<PathBuf, String> {
        let runtime_obj_path = target_dir.join("_track_runtime.o");
        let runtime_hash_path = target_dir.join("_track_runtime.hash");
        let runtime_hash = crate::yard::cache::BuildCache::compute_hash(crate::RUNTIME_C_SOURCE);
        if runtime_obj_path.exists()
            && fs::read_to_string(&runtime_hash_path)
                .is_ok_and(|cached_hash| cached_hash.trim() == runtime_hash)
        {
            return Ok(runtime_obj_path);
        }

        let runtime_c_path = target_dir.join("_track_runtime.c");
        let runtime_obj_temp_path = target_dir.join("_track_runtime.o.tmp");
        fs::write(&runtime_c_path, crate::RUNTIME_C_SOURCE)
            .map_err(|e| format!("Failed to write runtime helper source: {}", e))?;

        let status = Command::new("cc")
            .arg("-c")
            .arg(&runtime_c_path)
            .arg("-o")
            .arg(&runtime_obj_temp_path)
            .arg("-O3")
            .status()
            .map_err(|e| format!("Failed to compile runtime helper: {}", e))?;

        let _ = fs::remove_file(&runtime_c_path);

        if !status.success() {
            let _ = fs::remove_file(&runtime_obj_temp_path);
            return Err(format!(
                "Runtime helper compilation failed with code: {:?}",
                status.code()
            ));
        }

        fs::rename(&runtime_obj_temp_path, &runtime_obj_path)
            .map_err(|e| format!("Failed to install runtime helper object: {}", e))?;
        fs::write(&runtime_hash_path, runtime_hash)
            .map_err(|e| format!("Failed to write runtime helper cache key: {}", e))?;

        Ok(runtime_obj_path)
    }

    /// Link all compiled object files + runtime object into the final native executable.
    pub fn link_binary(
        obj_files: &[PathBuf],
        exe_path: &Path,
        target_dir: &Path,
    ) -> Result<(), String> {
        let runtime_obj = Self::get_or_compile_runtime(target_dir)?;

        let mut cmd = Command::new("cc");

        // Use fast linker flag if available
        if let Some(flag) = Self::detect_fast_linker_flag() {
            cmd.arg(flag);
        }

        for obj in obj_files {
            cmd.arg(obj);
        }
        cmd.arg(&runtime_obj);
        cmd.arg("-o").arg(exe_path).arg("-lm").arg("-no-pie");

        let status = cmd
            .status()
            .map_err(|e| format!("Linker invocation failed: {}", e))?;

        if !status.success() {
            return Err(format!("Linker failed with exit code: {:?}", status.code()));
        }

        Ok(())
    }
}
