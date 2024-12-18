# Moverr

Tired of manually moving your games between drives or uninstalling them once you
run out of space? Moverr is a simple tool that lets you move huge folders between
drives and create symbolic links to the original location.

This way you can keep your games installed and play them without having to worry
about running out of space.
It should work for any folder, but it was designed with games in mind.
It can replace tools like SteamMover.

File system operations are running in a separate thread, so you can keep using the
application while it's moving files and see the progress in real time.

:warning: **Warning**: This tool is still in early development and is
***not usable*** just yet.

## Usage

Run the application. This should show a simple TUI interface. You can access the
main menu by pressing `Esc`. From there, by using `File`›`Open`, you can select
the directory in which your games are installed.
Then, all your games should be listed in the main view.

## Current state

```mermaid
flowchart TD
    classDef complete fill: #080
    subgraph UI[User Interface]
        MUI --> MM
        MUI[Main UI] --> PVMT
        MUI --> DP
        subgraph P[Popups]
            DP[Dynamic pop-up system]
            DP --> POD[Project open dialog]
            POD -->|For now just text input for path| POD1[Tree - like UI for browsing files]
            MD[Move dialog] --> POD1
        end
        subgraph M[Menu]
            MM[Main menu] --> POD
            MM --> RO[Recently opened]
        end
        subgraph PV[Project view]
            PVMT[Main table] --> PVC[Controls]
            PVC --> MC[Controls for moving files]
            MC --> MD
        end
    end
    subgraph IO[Input/Output]
        PVMT --> AIO1[Async calculating file sizes]
        MC --> SL[Creating symlinks]
        SL --> AIO2[Async moving files]
        MC --> MB[Async moving files back]
        MB --> RSL[Removing symlinks]
    end

    class MUI complete
    class MM complete
    class POD complete
    class PVMT complete
    class PVC complete
    class AIO1 complete
    class AIO2 complete
    class DP complete
```