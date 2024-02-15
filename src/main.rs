use gpapi::Gpapi;
use tokio;
use std::path::Path;
use std::fs::File;
use std::env;
use std::process;
use axmldecoder::parse;
use axmldecoder::Node;
use std::collections::HashSet;
use std::io::Read;
use zip::read::ZipArchive;

#[tokio::main]
 async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <username> <password> <rebuilt_apk>", args[0]);
        process::exit(1);
    }
    let username = &args[1];
    let password = &args[2];
    let rebuilt_apk = &args[3];
    let version: i32 = get_android_version(rebuilt_apk).parse().unwrap();
    let mut gp = Gpapi::new("en_US", "UTC", "hero2lte"); 
    gp.login(username, password).await.unwrap();
    println!("File download started for Polkadot Vault v{}",version);
    gp.download("io.parity.signer", Some(version), false, true, &Path::new("./"), None).await.unwrap();
    let playstore_apk = "./io.parity.signer.apk";

    if compare_apks(rebuilt_apk, playstore_apk) {
        println!("APKs match!");
    } else {
        println!("APKs don't match!");
    }
}

fn get_android_version(rebuilt_apk: &str)-> String {
    let rebuilt_file = File::open(rebuilt_apk).expect("Failed to open the APK file");
    let mut rebuilt_zip = ZipArchive::new(rebuilt_file).expect("Failed to read the first APK file");
    let mut android_manifest = rebuilt_zip.by_name("AndroidManifest.xml").unwrap();
    let mut manifest_bytes = Vec::new();
    android_manifest.read_to_end(&mut manifest_bytes);
    let manifest_xml = axmldecoder::parse(&manifest_bytes).unwrap();
    let node = manifest_xml.get_root().as_ref().unwrap();
    match node {
        Node::Element(element) => {
            let att = element.get_attributes().get("android:versionCode").unwrap();
            return att.clone();
        },
        Node::Cdata(_) => todo!()
    }

}

fn compare_apks(first_apk: &str, second_apk: &str) -> bool {
    let first_file = File::open(first_apk).expect("Failed to open the first APK file");
    let second_file = File::open(second_apk).expect("Failed to open the second APK file");

    let mut first_zip = ZipArchive::new(first_file).expect("Failed to read the first APK file");
    let mut second_zip = ZipArchive::new(second_file).expect("Failed to read the second APK file");

    compare_entry_names(&mut first_zip, &mut second_zip) && compare_entry_contents(&mut first_zip, &mut second_zip)
}

fn compare_entry_names(first_zip: &mut ZipArchive<File>, second_zip: &mut ZipArchive<File>) -> bool {
    let ignore_files: HashSet<String> = [
        "META-INF/UPLOAD.RSA".into(),
        "META-INF/UPLOAD.SF".into(),
        "assets/Database/db".into(), //cold db for Vault. It changes on every build.
        "META-INF/MANIFEST.MF".into(),
        "META-INF/GOOGPLAY.RSA".into(),
        "META-INF/GOOGPLAY.SF".into(),
        "stamp-cert-sha256".into(),
        "assets/dexopt/baseline.profm".into()
    ].iter().cloned().collect();

    let mut first_names: Vec<String> = first_zip.file_names()
    .map(String::from)
    .filter(|name| !ignore_files.contains(name))
    .collect();
    let mut second_names: Vec<String> = second_zip.file_names()
    .map(String::from)
    .filter(|name| !ignore_files.contains(name))
    .collect();

    // Sort the file names
    first_names.sort();
    second_names.sort();

    if first_names != second_names {
        println!("Manifests differ in content or length");
        return false;
    }


    true
}

fn compare_entry_contents(first_zip: &mut ZipArchive<File>, second_zip: &mut ZipArchive<File>) -> bool {
    // Use the corrected ignore_files HashSet type and filtering logic
    let ignore_files: HashSet<String> = [
        "META-INF/UPLOAD.RSA".into(),
        "META-INF/UPLOAD.SF".into(),
        "assets/Database/db".into(),
        "META-INF/MANIFEST.MF".into(),
        "META-INF/GOOGPLAY.RSA".into(),
        "META-INF/GOOGPLAY.SF".into(),
        "stamp-cert-sha256".into(),
        "assets/dexopt/baseline.profm".into()
    ].iter().cloned().collect();

    let mut success = true;

    for i in 0..first_zip.len() {
        let mut first_entry = first_zip.by_index(i).expect("Failed to read entry from the first APK");
        if ignore_files.contains(&first_entry.name().to_string()) {
            continue;
        }

        match second_zip.by_name(first_entry.name()) {
            Ok(mut second_entry) => {
                let mut first_bytes = Vec::new();
                let mut second_bytes = Vec::new();
                first_entry.read_to_end(&mut first_bytes).expect("Failed to read bytes from the first APK");
                second_entry.read_to_end(&mut second_bytes).expect("Failed to read bytes from the second APK");
                if first_entry.name() == "AndroidManifest.xml" {
                    let res: axmldecoder::XmlDocument = parse(&first_bytes).unwrap();
                    let firstXml = res.to_string().unwrap();
                    let res2 = parse(&second_bytes).unwrap();
                    let secondXml = res2.to_string().unwrap(); 
                    if firstXml != secondXml {
                        println!("AndroidManifest.xml is different!");
                        success = false;
                    }
                }
                else {
                    
                    if first_bytes != second_bytes {
                    println!("APKs differ on file {}!", first_entry.name());
                    success = false;
                    }
                }
                
            },
            Err(_) => {
                println!("File {} not found in the second APK!", first_entry.name());
                success = false;
            },
        }
    }
    success
}
