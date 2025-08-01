# Dota Modding Community Patcher

Small utility tool to enable custom mods in Dota 2

## Usage
1. Launch the tool
2. Place mods in  `...\dota 2 beta\game\DotaModdingCommunityMods` folder
### Optionally
1. Move executable to dota folder (example: `...\steamapps\common\dota 2 beta`)
2. Add `cmd /c "start /b DMC_Dota2_Patcher.exe %command%"` at the **START** of Dota 2 launch options
3. Place mods in  `...\dota 2 beta\game\DotaModdingCommunityMods` folder
Every time you launch Dota 2 using steam, it will verify patch state(patch if needed) and launch the game
So you don't need to patch the game manually everytime it updates

# Revert to original files
Tool makes backup of gameinfo_branchspecific.gi and dota.signatures in their respective locations:
- `...\dota 2 beta\game\dota\gameinfo_branchspecific.gi_backup`
- `...\dota 2 beta\game\bin\win64\dota.signatures_backup`
#### Reverting to prepatch state:
1. Delete `gameinfo_branchspecific.gi` and `dota.signatures`
2. Remove "`_backup`" from the `gameinfo_branchspecific.gi_backup` and `dota.signatures_backup`
