use std::path::Path;

use reqwest::Client;
use serde::Deserialize;
use vendordeps::{CppDependency, JavaDependency, JniDependency};

const LATEST_VERSION: &'static str = "2025.1.1-beta-1";
const YEAR: u32 = 2025;

#[derive(Deserialize, Debug)]
struct FolderItem {
    name: String,
    folder: bool,
}

#[derive(Deserialize, Debug)]
struct Folder {
    data: Vec<FolderItem>,
}

async fn index_artifactory(client: &Client, base: &str, link: &str) {
    let wpilib_dir = Path::new("wpilib");
    _ = std::fs::create_dir_all(wpilib_dir);
    let folder: Folder = client
        .get(&format!("{}/{}/?recordNum=0", base, link))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    for item in folder.data {
        if !item.folder {
            continue;
        }
        let name = item.name;
        let folder: Folder = client
            .get(&format!("{}/{}/{}/?recordNum=0", base, link, &name))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let mut jni: Vec<(String, Vec<String>)> = Vec::new();
        let mut java: Vec<String> = Vec::new();
        let mut cpp: Vec<(String, Vec<String>)> = Vec::new();
        for item in folder.data {
            let artifact_id = item.name.as_str();
            if artifact_id == format!("{}-cpp", &name) {
                let mut support = Vec::new();
                let folder: Folder = client
                    .get(&format!(
                        "{}/{}/{}/{}/?recordNum=0",
                        base, link, &name, artifact_id
                    ))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                for item in folder.data {
                    let version = item.name.as_str();
                    if item.name == LATEST_VERSION {
                        let folder: Folder = client
                            .get(&format!(
                                "{}/{}/{}/{}/{}/?recordNum=0",
                                base, link, &name, artifact_id, version
                            ))
                            .send()
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap();
                        let expected_start = format!("{}-{}-", artifact_id, version);
                        for item in folder.data {
                            let zipname = item.name.as_str();
                            if zipname.ends_with("debug.zip")
                                || zipname.ends_with("debug.jar")
                                || zipname.ends_with("static.zip")
                                || zipname.ends_with("static.jar")
                                || zipname.ends_with("staticdebug.zip")
                                || zipname.ends_with("staticdebug.jar")
                                || zipname.ends_with("sources.zip")
                                || zipname.ends_with("sources.jar")
                                || zipname.ends_with("headers.zip")
                                || zipname.ends_with("headers.jar")
                            {
                                continue;
                            }
                            if zipname.starts_with(&expected_start) {
                                let ending = &zipname[expected_start.len()..zipname.len() - 4];
                                support.push(ending.to_string());
                            }
                        }
                    }
                }
                if !support.is_empty() {
                    cpp.push((artifact_id.to_string(), support));
                }
            } else if item.name == format!("{}-java", &name) {
                java.push(artifact_id.to_string());
            } else if item.name == format!("{}-jni", &name) {
                let mut support = Vec::new();
                let folder: Folder = client
                    .get(&format!(
                        "{}/{}/{}/{}/?recordNum=0",
                        base, link, &name, artifact_id
                    ))
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                for item in folder.data {
                    let version = item.name.as_str();
                    if item.name == LATEST_VERSION {
                        let folder: Folder = client
                            .get(&format!(
                                "{}/{}/{}/{}/{}/?recordNum=0",
                                base, link, &name, artifact_id, version
                            ))
                            .send()
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap();
                        let expected_start = format!("{}-{}-", artifact_id, version);
                        for item in folder.data {
                            let zipname = item.name.as_str();
                            if zipname.ends_with("debug.zip")
                                || zipname.ends_with("debug.jar")
                                || zipname.ends_with("static.zip")
                                || zipname.ends_with("static.jar")
                                || zipname.ends_with("staticdebug.zip")
                                || zipname.ends_with("staticdebug.jar")
                                || zipname.ends_with("sources.zip")
                                || zipname.ends_with("sources.jar")
                                || zipname.ends_with("headers.zip")
                                || zipname.ends_with("headers.jar")
                            {
                                continue;
                            }
                            if zipname.starts_with(&expected_start) {
                                let ending = &zipname[expected_start.len()..zipname.len() - 4];
                                support.push(ending.to_string());
                            }
                        }
                    }
                }
                if !support.is_empty() {
                    jni.push((artifact_id.to_string(), support));
                }
            }
        }
        if cpp.is_empty() && java.is_empty() && jni.is_empty() {
            continue
        }
        let file_name = format!("wpilib-{}.json", name);
        let vendordep = vendordeps::VendorDep {
            file_name: file_name.clone(),
            version: LATEST_VERSION.to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            frc_year: YEAR,
            maven_urls: vec!["https://frcmaven.wpi.edu/artifactory/release/".to_string()],
            json_url: format!("https://raw.githubusercontent.com/wilsonwatson/vendordeps/main/wpilib/{}", file_name),
            conflicts_with: vec![],
            java_dependencies: java.into_iter().map(|x| JavaDependency {
                group_id: format!("edu.wpi.first.{}", name),
                artifact_id: x,
                version: LATEST_VERSION.to_string(),
            }).collect(),
            cpp_dependencies: cpp.into_iter().map(|(x, d)| CppDependency {
                group_id: format!("edu.wpi.first.{}", name),
                artifact_id: x,
                version: LATEST_VERSION.to_string(),
                header_classifier: "headers".to_string(),
                binary_platforms: d
            }).collect(),
            jni_dependencies: jni.into_iter().map(|(x, d)| JniDependency {
                group_id: format!("edu.wpi.first.{}", name),
                artifact_id: x,
                version: LATEST_VERSION.to_string(),
                is_jar: true, /* TODO: detect this */
                skip_invalid_platforms: true,
                valid_platforms: d,
                sim_mode: None,
            }).collect(),
        };
        let vendordep = serde_json::to_string_pretty(&vendordep).unwrap();
        std::fs::write(wpilib_dir.join(file_name), vendordep).unwrap();
    }
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    index_artifactory(
        &client,
        "https://frcmaven.wpi.edu/ui/api/v1/ui/v2/nativeBrowser/release",
        "edu/wpi/first",
    )
    .await;
}
