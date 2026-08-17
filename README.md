# straycat-rs ![build](https://github.com/UtaUtaUtau/straycat-rs/actions/workflows/build.yml/badge.svg)
 A Rust port of straycat, a WORLD-based UTAU resampler

# How to use
 Download the [latest version](https://github.com/UtaUtaUtau/straycat-rs/releases/latest/download/straycat-rs.exe) of straycat-rs and use it like a regular UTAU resampler.
# How to compile
 **Note**: By the nature of an UTAU resampler, it is only ideal to build this program in Windows.
 1. Install [rustup](https://rustup.rs/).
 2. Decide whether you want to build with the icon.
    - Build with icon:
        1. Install [Windows SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/).
        2. Locate `rc.exe`. It is usually in `C:\Program Files (x86)\Windows Kits\10\bin\<version number>\x64\rc.exe`
        3. Replace the location for `rc.exe` in the build script `build.rs`.
        4. Build with `cargo build -r`
    - Build without icon:
        1. Delete the build script `build.rs`.
        2. Build with `cargo build -r`
 
 I highly encourage building in the other platforms as those builds can be used in [OpenUtau.](https://github.com/stakira/OpenUtau) Build steps for Mac/Linux should be similar, just follow build without icon skipping step 1.
# Flag Documentation
Check flag documentation [here](flag_docs.md).

An official resampler manifest file is now available for OpenUtau users [here.](https://raw.githubusercontent.com/UtaUtaUtau/straycat-rs/master/straycat-rs.yaml) Right click and select `Save as...`
# Example Renders
 These renders use straycat-rs 1.1.0. No flags are used in these renders unless stated.
 
 **Voicebank**: 電圧空 -Halcyon- / Denatsu Sora -Halcyon- / VCV
 
https://github.com/user-attachments/assets/4e9db61c-7b84-48f3-a558-458a9bd913aa

 **Voicebank**: 紅 通常 / Kurenai Normal / VCV

https://github.com/user-attachments/assets/8ebd470a-17f3-4c15-9bbd-c3d3707edcf1

 **Voicebank**: 戯白メリー Highwire / Kohaku Merry Highwire / VCV

https://github.com/user-attachments/assets/ec79b1f5-6e6b-4dfa-bb77-8e01ae8a7cdc

 **Voicebank**: 水音ラル float / Mine Laru float / VCV

https://github.com/user-attachments/assets/ba31dc4a-83e7-4683-8a70-922cec341bb1

 **Voicebank**: 吼音ブシ-武- / Quon Bushi -武- / VCV

https://github.com/user-attachments/assets/56ab2a27-0780-49ea-96fc-5f56f7838a0a

 **Voicebank**: 廻音シュウVer1.00 / Mawarine Shuu Ver1.00 / VCV

https://github.com/user-attachments/assets/08dcba09-e5d8-4ed6-a7a0-6a7d16bf1464

 **Voicebank**: Number Bronze・ate / CVVC

https://github.com/user-attachments/assets/01eb7cc6-d178-4f1d-910a-1fd312c0ee2d

 **Voicebank**: 学人デシマル χΩ / Gakuto Deshimaru Chi-Omega / CVVC

https://github.com/user-attachments/assets/692a2533-5fc2-4da5-8c50-78216c0851eb

 **Voicebank**: CZloid / English VCCV / Uses P0p-1 for CCs

https://github.com/user-attachments/assets/bf226d88-5692-4e0b-bf22-58893e52ff51
 
# Remarks
 This resampler will not be an exact copy of [straycat](https://github.com/UtaUtaUtau/straycat), but a variation of it. It may not do the exact same things as straycat, but my goal with this resampler is to match or surpass the quality of straycat.

 I am also not obliged to transfer the flags from straycat to straycat-rs, but if I do, I will most likely add improvements to it to give the users a better experience.

 Overall, this resampler serves to be a new and improved version of the older Python-based straycat, not a faithful translation of straycat to a compiled language.
