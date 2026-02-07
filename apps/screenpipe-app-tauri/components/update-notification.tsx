import React, { useEffect, useState } from "react";
import { sendNotification } from "@tauri-apps/plugin-notification";
import { platform } from "@tauri-apps/plugin-os";

const UpdateNotification: React.FC<{ checkIntervalHours: number }> = ({
  checkIntervalHours = 3,
}) => {
  useEffect(() => {
    const checkForUpdates = async () => {
      let lastCheckTime: string | null = null;
      try { lastCheckTime = localStorage?.getItem("lastUpdateCheckTime"); } catch {}
      const currentTime = Date.now();

      if (
        !lastCheckTime ||
        currentTime - parseInt(lastCheckTime) > checkIntervalHours * 3600000
      ) {
        const os = platform();
        const releasePageUrl =
          "https://github.com/screenpipe/screenpipe/releases/latest";

        try {
          // Get the latest release info from GitHub API
          const apiUrl = "https://api.github.com/repos/screenpipe/screenpipe/releases/latest";
          const response = await fetch(apiUrl);
          const release = await response.json();

          if (!release.assets) {
            console.error("No assets found in release");
            return;
          }

          // Extract download links from release assets
          const assets = release.assets.map((asset: any) => asset.browser_download_url);

          let downloadLink = "";
          if (os === "windows") {
            downloadLink =
              assets.find((link: string) => link.includes("nsis") || link.includes(".exe")) || "";
          } else if (os === "macos") {
            // For macOS, provide both ARM and Intel options
            const armLink =
              assets.find((link: string) => link.includes("aarch64") && link.includes(".dmg")) || "";
            const intelLink =
              assets.find((link: string) => link.includes("x86_64") && link.includes(".dmg")) || "";
            if (armLink || intelLink) {
              downloadLink = `Download: ${releasePageUrl}`;
            }
          }

          if (downloadLink) {
            sendNotification({
              title: "Screenpipe Update Available",
              body: `A new version of Screenpipe is available. ${downloadLink}`,
            });
          }
        } catch (error) {
          console.error("Error checking for updates:", error);
        }

        try { localStorage?.setItem("lastUpdateCheckTime", currentTime.toString()); } catch {}
      }
    };

    checkForUpdates();
    const interval = setInterval(checkForUpdates, checkIntervalHours * 3600000);

    return () => clearInterval(interval);
  }, [checkIntervalHours]);

  return null;
};

export default UpdateNotification;
