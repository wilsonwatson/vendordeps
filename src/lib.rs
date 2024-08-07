#![deny(rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../README.md")]

#[allow(unused)]
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[doc = "Latest release version of WPILib."]
pub const WPILIB_LATEST_VERSION: &'static str = "2024.3.2";
#[doc = "Maven repository for WPILib releases."]
pub const WPILIB_RELEASE_MAVEN_REPO: &'static str = "https://frcmaven.wpi.edu/artifactory/release/";

#[cfg(feature = "download")]
pub mod error;
#[cfg(feature = "download")]
pub use error::Result;

#[doc = "A reference to another vendordep."]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageSpec {
    #[doc = "The `uuid` field of the other vendordep."]
    pub uuid: String,
    #[doc = "The message printed if this package is also included."]
    pub error_message: String,
    #[doc = "File name of resulting JSON file."]
    pub offline_file_name: String,
}

#[doc = "A dependency for Java Compilation."]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDependency {
    #[doc = "Maven group."]
    pub group_id: String,
    #[doc = "Maven artifact."]
    pub artifact_id: String,
    #[doc = "Maven version."]
    pub version: String,
}

impl JavaDependency {
    #[doc = "Resolve Maven URL."]
    pub fn get_url(&self, maven_url: &str) -> String {
        format!(
            "{0}{1}/{2}/{3}/{4}",
            maven_url,
            self.group_id.replace('.', "/"),
            self.artifact_id,
            self.version,
            self.file_name()
        )
    }

    #[doc = "Get name of jar file."]
    pub fn file_name(&self) -> String {
        format!("{}-{}.jar", self.artifact_id, self.version)
    }

    #[cfg(feature = "download")]
    #[doc = "Download Maven artifact and save it in a directory."]
    pub async fn download_library_to_folder<P: AsRef<Path>>(
        &self,
        out_folder: P,
        maven_url: &str,
    ) -> Result<()> {
        let url = self.get_url(maven_url);
        let res = reqwest::get(url).await?.bytes().await?.to_vec();
        _ = std::fs::create_dir_all(out_folder.as_ref());
        std::fs::write(out_folder.as_ref().join(self.file_name()), res)?;
        Ok(())
    }
}

#[doc = "A native dependency required for Java."]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JniDependency {
    #[doc = "Maven group."]
    pub group_id: String,
    #[doc = "Maven artifact."]
    pub artifact_id: String,
    #[doc = "Maven version."]
    pub version: String,
    #[doc = "Whether or not the artifact is in a `.jar` file. If false, looks for a `.zip` file instead."]
    pub is_jar: bool,
    // Idk what this does
    pub skip_invalid_platforms: bool,
    pub valid_platforms: Vec<String>,
    // Idk what this does
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sim_mode: Option<String>,
}

impl JniDependency {
    #[doc = "Resolve Maven URL."]
    pub fn get_url(&self, maven_url: &str, platform: &str, is_debug: bool) -> String {
        format!(
            "{0}{1}/{2}/{3}/{2}-{3}-{4}{5}.{6}",
            maven_url,
            self.group_id.replace('.', "/"),
            self.artifact_id,
            self.version,
            platform,
            if is_debug { "debug" } else { "" },
            if self.is_jar { "jar" } else { "zip" },
        )
    }

    #[cfg(feature = "download")]
    #[doc = "Download Maven artifact and unzip it to a directory."]
    pub async fn download_library_to_folder<P: AsRef<Path>>(
        &self,
        out_folder: P,
        maven_url: &str,
        platform: BinaryPlatform,
        is_debug: bool,
    ) -> Result<()> {
        let url = self.get_url(maven_url, platform.to_str(), is_debug);
        let res = std::io::Cursor::new(reqwest::get(url).await?.bytes().await?.to_vec());
        let mut zip = zip::ZipArchive::new(res)?;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            if f.name().ends_with("/") {
                continue;
            }
            let outpath = out_folder.as_ref().join(
                f.enclosed_name()
                    .ok_or_else(|| error::Error::ZipSecurityError)?,
            );
            _ = std::fs::create_dir_all(outpath.parent().unwrap());
            let mut outf = std::fs::File::create(outpath)?;
            std::io::copy(&mut f, &mut outf)?;
        }
        Ok(())
    }
}

macro_rules! binary_platform {
    ($name:ident {$($variant:ident = $val:literal),* $(,)?}) => {
        #[doc = "Valid platforms for WPILib execution."]
        #[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $(
                #[serde(rename = $val)]
                $variant,
            )*
        }

        impl $name {
            pub fn to_str(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $val,
                    )*
                }
            }
        }
    };
}

binary_platform!(BinaryPlatform {
    LinuxArm32 = "linuxarm32",
    LinuxArm64 = "linuxarm64",
    LinuxAthena = "linuxathena",
    LinuxX86_64 = "linuxx86-64",
    OsxUniversal = "osxuniversal",
    WindowsArm64 = "windowsarm64",
    WindowsX86_64 = "windowsx86-64",
    Headers = "headers",
});

#[doc = "A dependency for C++ compilation."]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CppDependency {
    #[doc = "Maven group."]
    pub group_id: String,
    #[doc = "Maven artifact."]
    pub artifact_id: String,
    #[doc = "Maven version."]
    pub version: String,
    #[doc = "Instead of shipping headers with individual platform artifacts, headers are stored in a separate artifact. This value is used in place of the 'platform' to get the url."]
    pub header_classifier: String,
    #[doc = "Platforms this library supports. May be empty."]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binary_platforms: Vec<String>,
}

impl CppDependency {
    #[doc = "Resolve Maven URL."]
    pub fn get_url(
        &self,
        maven_url: &str,
        platform: &str,
        is_static: bool,
        is_debug: bool,
    ) -> String {
        format!(
            "{0}{1}/{2}/{3}/{2}-{3}-{4}{5}{6}.zip",
            maven_url,
            self.group_id.replace('.', "/"),
            self.artifact_id,
            self.version,
            platform,
            if is_static { "static" } else { "" },
            if is_debug { "debug" } else { "" }
        )
    }

    #[cfg(feature = "download")]
    #[doc = "Download Maven artifact and unzip it to a directory."]
    pub async fn download_library_to_folder<P: AsRef<Path>>(
        &self,
        out_folder: P,
        maven_url: &str,
        platform: BinaryPlatform,
        is_static: bool,
        is_debug: bool,
    ) -> Result<()> {
        let url = self.get_url(maven_url, platform.to_str(), is_static, is_debug);
        let res = std::io::Cursor::new(reqwest::get(url).await?.bytes().await?.to_vec());
        let mut zip = zip::ZipArchive::new(res)?;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            if f.name().ends_with("/") {
                continue;
            }
            let outpath = out_folder.as_ref().join(
                f.enclosed_name()
                    .ok_or_else(|| error::Error::ZipSecurityError)?,
            );
            _ = std::fs::create_dir_all(outpath.parent().unwrap());
            let mut outf = std::fs::File::create(outpath)?;
            std::io::copy(&mut f, &mut outf)?;
        }
        Ok(())
    }

    #[cfg(feature = "download")]
    #[doc = "Download headers and unzip them to a directory."]
    pub async fn download_headers_to_folder<P: AsRef<Path>>(
        &self,
        out_folder: P,
        maven_url: &str,
    ) -> Result<()> {
        self.download_library_to_folder(
            out_folder,
            maven_url,
            BinaryPlatform::Headers,
            false,
            false,
        )
        .await
    }
}

#[doc = "Info needed for C++ compilation. Retrieved as a result of [`VendorDep::download_all_cpp_deps_to_folder`]."]
#[derive(Debug, Clone)]
pub struct CppInfo {
    #[doc = "Root directories containing headers."]
    pub include_dirs: Vec<PathBuf>,
    #[doc = "Directories containing library objects."]
    pub library_search_paths: Vec<PathBuf>,
    #[doc = "Library names."]
    pub libraries: Vec<String>,
}

impl CppInfo {
    #[doc = "Create new [`CppInfo`] with no include directories or libraries."]
    pub fn new_empty() -> Self {
        Self {
            include_dirs: vec![],
            library_search_paths: vec![],
            libraries: vec![],
        }
    }

    #[cfg(feature = "download")]
    #[doc = "Create new [`CppInfo`] from existing directory structure generated by [`VendorDep::download_all_cpp_deps_to_folder`]."]
    pub fn from_existing<P: AsRef<Path>>(p: P) -> Result<Self> {
        let p = p.as_ref();
        let mut include_dirs = Vec::new();
        let mut library_search_paths = Vec::new();
        let mut libraries = Vec::new();
        for item in std::fs::read_dir(p)? {
            let item = item?;
            include_dirs.push(item.path().join("include"));
            let mut temp_search_paths = HashSet::new();
            for item in jwalk::WalkDir::new(item.path().join("libs")) {
                let item = item?;
                if let Some(stem) = item.path().file_stem() {
                    let stem = stem.to_string_lossy();
                    match item.path().extension().and_then(|x| x.to_str()) {
                        Some("so") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem[3..].to_string());
                        }
                        Some("dll") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem.to_string());
                        }
                        _ => {}
                    }
                }
            }
            library_search_paths.extend(temp_search_paths);
        }
        Ok(Self {
            include_dirs,
            library_search_paths,
            libraries,
        })
    }

    #[doc = "Combine another [`CppInfo`] value into this one."]
    pub fn extend(&mut self, other: Self) {
        self.include_dirs.extend(other.include_dirs);
        self.library_search_paths.extend(other.library_search_paths);
        self.libraries.extend(other.libraries);
    }

    #[doc = "Get `LD_LIBRARY_PATH` environment variable for runtime linking."]
    pub fn ld_library_path(&self) -> String {
        self.library_search_paths
            .iter()
            .map(|x| format!("{}", x.display()))
            .collect::<Vec<_>>()
            .join(":")
    }

    #[doc = "Get command line arguments passed to either `gcc` or `clang` for include directories."]
    pub fn gcc_clang_include_dir_args<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.include_dirs
            .iter()
            .map(|x| format!("-I{}", x.display()))
    }

    #[doc = "Get command line arguments passed to either `gcc` or `clang` for library search paths."]
    pub fn gcc_clang_library_search_path_args<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.library_search_paths
            .iter()
            .map(|x| format!("-L{}", x.display()))
    }

    #[doc = "Get command line arguments passed to either `gcc` or `clang` for libraries."]
    pub fn gcc_clang_library_args<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.libraries.iter().map(|x| format!("-l{}", x))
    }

    #[doc = "Get command line arguments passed to either `gcc` or `clang`. "]
    #[doc = "A combination of [`Self::gcc_clang_include_dir_args`], [`Self::gcc_clang_library_search_path_args`], and [`Self::gcc_clang_library_args`]."]
    pub fn gcc_clang_args<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        self.gcc_clang_include_dir_args()
            .chain(self.gcc_clang_library_search_path_args())
            .chain(self.gcc_clang_library_args())
    }
}

#[doc = "Vendor Dependency Format."]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorDep {
    #[doc = "File name that GradleRIO will write to `vendordeps/` directory."]
    pub file_name: String,
    #[doc = "Name of vendor library."]
    pub name: String,
    #[doc = "Vendor library version. Usually is the same as each artifact's Maven version."]
    pub version: String,
    #[doc = "Supported year."]
    #[serde(deserialize_with = "__private::deserialize_string_or_u32_for_u32")]
    pub frc_year: u32,
    #[doc = "UUID used for checking compatibility."]
    pub uuid: String,
    #[doc = "List of Maven repositories to search for Maven artifacts."]
    pub maven_urls: Vec<String>,
    #[doc = "URL for this. If up to date, the contents of the url should reproduce this [`VendorDep`] value."]
    pub json_url: String,
    #[doc = "A list of other [`VendorDep`]s this is explicitly incompatible with. Generally this includes older versions which would introduce name collisions."]
    #[serde(default)]
    pub conflicts_with: Vec<PackageSpec>,
    #[doc = "A list of Java source dependencies."]
    pub java_dependencies: Vec<JavaDependency>,
    #[doc = "A list of Java native dependencies."]
    pub jni_dependencies: Vec<JniDependency>,
    #[doc = "A list of C++ dependencies."]
    pub cpp_dependencies: Vec<CppDependency>,
}

impl VendorDep {
    #[cfg(feature = "download")]
    #[doc = "Download JSON from url and parse it."]
    pub async fn from_url(url: &str) -> Result<Self> {
        Ok(reqwest::get(url).await?.json::<Self>().await?)
    }

    #[cfg(feature = "download")]
    #[doc = "Download all cpp dependencies. Directory structure follows `<output_folder>/<cpp_dependency_name>/(lib|include)`."]
    pub async fn download_all_cpp_deps_to_folder<P: AsRef<Path>>(
        &self,
        p: P,
        binary_platform: BinaryPlatform,
        is_static: bool,
        is_debug: bool,
        skip_failed_packages: bool,
    ) -> Result<CppInfo> {
        let path = p.as_ref();
        let mut include_dirs = Vec::new();
        let mut library_search_paths = Vec::new();
        let mut libraries = Vec::new();
        for dep in &self.cpp_dependencies {
            let dep_path = path.join(&dep.artifact_id);
            let header_path = dep_path.join("include");
            'outer: loop {
                for maven_url in &self.maven_urls {
                    match dep
                        .download_headers_to_folder(&header_path, maven_url.as_str())
                        .await
                    {
                        Ok(_) => break 'outer,
                        _ => {}
                    }
                }
                if !skip_failed_packages {
                    return Err(crate::error::Error::NotFoundError(format!(
                        "{}:{}:{}",
                        dep.group_id, dep.artifact_id, dep.version
                    )));
                }
            }
            include_dirs.push(header_path);
            let libs_path = dep_path.join("libs");
            'outer: loop {
                for maven_url in &self.maven_urls {
                    match dep
                        .download_library_to_folder(
                            &libs_path,
                            maven_url.as_str(),
                            binary_platform,
                            is_static,
                            is_debug,
                        )
                        .await
                    {
                        Ok(_) => break 'outer,
                        _ => {}
                    }
                }
                if !skip_failed_packages {
                    return Err(crate::error::Error::NotFoundError(format!(
                        "{}:{}:{}",
                        dep.group_id, dep.artifact_id, dep.version
                    )));
                }
            }
            let mut temp_search_paths = HashSet::new();
            for item in jwalk::WalkDir::new(libs_path) {
                let item = item?;
                if let Some(stem) = item.path().file_stem() {
                    let stem = stem.to_string_lossy();
                    match item.path().extension().and_then(|x| x.to_str()) {
                        Some("so") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem[3..].to_string());
                        }
                        Some("dll") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem.to_string());
                        }
                        _ => {}
                    }
                }
            }
            library_search_paths.extend(temp_search_paths);
        }
        Ok(CppInfo {
            include_dirs,
            library_search_paths,
            libraries,
        })
    }

    #[cfg(feature = "download")]
    #[doc = "Download all JNI dependencies. Directory structure follows `<output_folder>/<cpp_dependency_name>/`."]
    pub async fn download_all_jni_deps_to_folder<P: AsRef<Path>>(
        &self,
        p: P,
        binary_platform: BinaryPlatform,
        is_debug: bool,
        skip_failed_packages: bool,
    ) -> Result<CppInfo> {
        let path = p.as_ref();
        let mut library_search_paths = Vec::new();
        let mut libraries = Vec::new();
        for dep in &self.jni_dependencies {
            let dep_path = path.join(&dep.artifact_id);
            'outer: loop {
                for maven_url in &self.maven_urls {
                    match dep
                        .download_library_to_folder(
                            &dep_path,
                            maven_url.as_str(),
                            binary_platform,
                            is_debug,
                        )
                        .await
                    {
                        Ok(_) => break 'outer,
                        _ => {}
                    }
                }
                if !skip_failed_packages {
                    return Err(crate::error::Error::NotFoundError(format!(
                        "{}:{}:{}",
                        dep.group_id, dep.artifact_id, dep.version
                    )));
                }
            }
            let mut temp_search_paths = HashSet::new();
            for item in jwalk::WalkDir::new(dep_path) {
                let item = item?;
                if let Some(stem) = item.path().file_stem() {
                    let stem = stem.to_string_lossy();
                    match item.path().extension().and_then(|x| x.to_str()) {
                        Some("so") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem[3..].to_string());
                        }
                        Some("dll") => {
                            temp_search_paths.insert(item.parent_path().to_path_buf());
                            libraries.push(stem.to_string());
                        }
                        _ => {}
                    }
                }
            }
            library_search_paths.extend(temp_search_paths);
        }
        Ok(CppInfo {
            include_dirs: vec![],
            library_search_paths,
            libraries,
        })
    }

    #[cfg(feature = "download")]
    #[doc = "Download all java dependencies. Note this does *not* include JNI dependencies. Directory structure follows `<output_folder>/<java_dependency_name>-<java_dependency_version>.jar`."]
    pub async fn download_all_java_deps_to_folder<P: AsRef<Path>>(
        &self,
        p: P,
        skip_failed_packages: bool,
    ) -> Result<Vec<PathBuf>> {
        let path = p.as_ref();
        for dep in &self.java_dependencies {
            'outer: loop {
                for maven_url in &self.maven_urls {
                    match dep.download_library_to_folder(path, maven_url).await {
                        Ok(_) => break 'outer,
                        _ => {}
                    };
                }
                if !skip_failed_packages {
                    return Err(crate::error::Error::NotFoundError(format!(
                        "{}:{}:{}",
                        dep.group_id, dep.artifact_id, dep.version
                    )));
                }
            }
        }

        Ok(std::fs::read_dir(path)?
            .into_iter()
            .filter_map(|x| x.ok())
            .filter_map(|x| {
                if x.file_type().ok()?.is_file() {
                    Some(x.path())
                } else {
                    None
                }
            })
            .collect())
    }
}

#[doc(hidden)]
mod __private {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize_string_or_u32_for_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Inner {
            Int(u32),
            String(String),
        }

        let res = Inner::deserialize(deserializer)?;
        match res {
            Inner::Int(x) => Ok(x),
            Inner::String(x) => x
                .parse()
                .map_err(|x| <D::Error as serde::de::Error>::custom(x)),
        }
    }
}

#[allow(unused)]
#[cfg(test)]
mod test {
    use tempfile::tempdir;

    use crate::VendorDep;

    #[cfg(feature = "download")]
    #[test]
    fn ctre_2024_headers() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let res = VendorDep::from_url("https://maven.ctr-electronics.com/release/com/ctre/phoenix6/latest/Phoenix6-frc2024-latest.json").await;
                assert!(res.is_ok(), "Failed to download from url");
                let ctre_vendordep = res.unwrap();
                let temp_dir = tempdir().unwrap();
                let res = ctre_vendordep.cpp_dependencies[0].download_headers_to_folder(temp_dir.path(), &ctre_vendordep.maven_urls[0]).await;
                assert!(res.is_ok(), "Failed to download headers");
                assert!(temp_dir.path().join("ctre/phoenix6/CANcoder.hpp").exists(), "Did not unzip properly!");
            })
    }
}
