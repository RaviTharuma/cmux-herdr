# [#](#title)iOS App

# [#](https://cmux-docs-release.vercel.app/docs/ios#title)iOS App

The cmux iOS app is a companion for your Mac. Pair your iPhone or iPad with a Mac running cmux and attach to your terminals from your phone, with optional forwarding of terminal notifications.

The iOS app is in beta. It ships on TestFlight as cmux BETA.

## [#](https://cmux-docs-release.vercel.app/docs/ios#access)Getting access

The app is distributed through TestFlight. Early access is included with [cmux Founders Edition](https://github.com/manaflow-ai/cmux#founders-edition), alongside other early features.

## [#](https://cmux-docs-release.vercel.app/docs/ios#prerequisites)Prerequisites

Before you start, you need:

-   A Mac running cmux and signed in.
-   An iPhone or iPad with the cmux BETA app installed from TestFlight.
-   A network path from your phone to your Mac (see Bring your own networking below).

## [#](https://cmux-docs-release.vercel.app/docs/ios#networking)Bring your own networking

cmux does not provide networking out of the box. The terminal stream flows directly between your phone and your Mac, so your phone has to be able to reach your Mac on the network. The simplest way is a private overlay network.

### [#](https://cmux-docs-release.vercel.app/docs/ios#tailscale)Tailscale (recommended)

Install [Tailscale](https://tailscale.com) on both your Mac and your phone and sign in to the same tailnet. Your phone can then reach your Mac by its tailnet address from anywhere, with no port forwarding.

### [#](https://cmux-docs-release.vercel.app/docs/ios#wireguard)WireGuard

If you already run [WireGuard](https://www.wireguard.com), put your Mac and phone on the same network and use the Mac's WireGuard address.

Whichever you choose, the connection is yours. cmux does not proxy or relay terminal traffic between your devices.

## [#](https://cmux-docs-release.vercel.app/docs/ios#pair)Pair and connect

1.  On the Mac, open the Mobile Connect window in cmux.
2.  On your phone, open the cmux BETA app and confirm the pairing shown in Mobile Connect.
3.  Once paired, your workspaces and terminals appear on the phone over your network.

Pairing is per device. A new device, or a reinstalled app, pairs again.

## [#](https://cmux-docs-release.vercel.app/docs/ios#notifications)Notifications

You can opt in to forward terminal notifications to your phone, so a pane that needs attention pushes to your device. When forwarding is on, the notification text (title and body, drawn from terminal output) is sent through cmux servers to Apple's push service to deliver it. Turn on Hide content in the notification settings to send a generic message instead and keep terminal text on your devices. See [notifications](https://cmux-docs-release.vercel.app/docs/notifications) for how cmux raises them.

## [#](https://cmux-docs-release.vercel.app/docs/ios#data)What data is stored

cmux is built so your terminal stays between your devices. On cmux servers we store only what is needed to sign you in and connect your devices:

-   Your account from sign-in (email), to identify you across devices.
-   A push token for your device, to deliver forwarded terminal notifications.
-   Pairing and device metadata, to connect your phone to your Mac.

Your interactive terminal session, its contents and keystrokes, flows directly between your phone and your Mac over your own network, and is not relayed through or stored on cmux servers. The one exception is forwarded notification text, which transits cmux and Apple's push service to reach your phone, unless you enable Hide content (see Notifications above).

## [#](https://cmux-docs-release.vercel.app/docs/ios#enterprise)Enterprise and self-hosted

For a self-hosted or air-gapped deployment, contact [founders@manaflow.com](mailto:founders@manaflow.com?subject=cmux%20enterprise).

[SSH](https://cmux-docs-release.vercel.app/docs/ssh) [Remote tmux](https://cmux-docs-release.vercel.app/docs/remote-tmux)

Canonical: https://cmux-docs-release.vercel.app/docs/ios
