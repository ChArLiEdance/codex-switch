import AppKit
import SwiftUI

public typealias TrayCallback = @convention(c) (
    UnsafePointer<CChar>?,
    UnsafePointer<CChar>?
) -> Void

private struct TrayPayload: Codable {
    var locale: String?
    var currentProfile: String?
    var currentTitle: String?
    var currentQuota: QuotaSummary?
    var profiles: [TrayProfileEntry]?
}

private struct TrayProfileEntry: Codable, Identifiable {
    var folderName: String
    var displayTitle: String
    var nickname: String
    var planName: String?
    var quota: QuotaSummary
    var status: String
    var authPresent: Bool

    var id: String { folderName }

    var menuTitle: String {
        let preferred = nickname.trimmingCharacters(in: .whitespacesAndNewlines)
        return preferred.isEmpty ? displayTitle : preferred
    }
}

private struct QuotaSummary: Codable {
    var fiveHour: QuotaWindow
    var weekly: QuotaWindow
}

private struct QuotaWindow: Codable {
    var remainingPercent: UInt8?
    var refreshAt: String?
    var resetAtTimestamp: Int64?
}

private struct TrayLabels {
    let show: String
    let current: String
    let switchAccounts: String
    let settings: String
    let about: String
    let quit: String
    let noAccount: String
    let fiveHour: String
    let weekly: String
    let used: String
    let left: String
    let resets: String
    let noQuota: String

    static func resolve(locale: String?) -> TrayLabels {
        if locale?.hasPrefix("zh") == true {
            return TrayLabels(
                show: "显示主界面",
                current: "当前账号",
                switchAccounts: "切换账号",
                settings: "设置",
                about: "关于",
                quit: "退出",
                noAccount: "暂无当前账号",
                fiveHour: "5h",
                weekly: "7d",
                used: "已用",
                left: "剩余",
                resets: "重置",
                noQuota: "暂无额度数据"
            )
        }
        return TrayLabels(
            show: "Show Main Window",
            current: "Current Account",
            switchAccounts: "Switch Account",
            settings: "Settings",
            about: "About",
            quit: "Quit",
            noAccount: "No active account",
            fiveHour: "5h",
            weekly: "7d",
            used: "Used",
            left: "Left",
            resets: "Resets",
            noQuota: "No quota data"
        )
    }
}

@MainActor
private final class CodexSwitchNativeTrayController: NSObject, NSMenuDelegate {
    static let shared = CodexSwitchNativeTrayController()

    private var callback: TrayCallback?
    private var payload = TrayPayload()
    private var statusItem: NSStatusItem?
    private var templateIcon: NSImage?

    func install(iconBytes: UnsafePointer<UInt8>?, iconLength: Int, callback: TrayCallback?) {
        self.callback = callback
        if let iconBytes, iconLength > 0 {
            self.templateIcon = Self.makeTemplateIcon(bytes: Array(UnsafeBufferPointer(start: iconBytes, count: iconLength)))
        }

        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        item.autosaveName = "codex-switch-main"
        item.button?.image = self.templateIcon
        item.button?.imagePosition = .imageOnly
        item.button?.toolTip = "Codex Switch"
        self.statusItem = item
        self.rebuildMenu()
    }

    func sync(json: String) {
        guard let data = json.data(using: .utf8) else { return }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        if let nextPayload = try? decoder.decode(TrayPayload.self, from: data) {
            self.payload = nextPayload
            self.rebuildMenu()
        }
    }

    private func rebuildMenu() {
        let labels = TrayLabels.resolve(locale: self.payload.locale)
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.delegate = self

        menu.addItem(self.makeQuotaCardItem(labels: labels))
        menu.addItem(.separator())
        menu.addItem(self.actionItem(title: labels.show, action: #selector(self.showMain(_:))))
        menu.addItem(.separator())
        menu.addItem(self.makeSwitchMenuItem(labels: labels))
        menu.addItem(.separator())
        menu.addItem(self.actionItem(title: labels.settings, action: #selector(self.openSettings(_:))))
        menu.addItem(self.actionItem(title: labels.about, action: #selector(self.openAbout(_:))))
        menu.addItem(.separator())
        menu.addItem(self.actionItem(title: labels.quit, action: #selector(self.quit(_:))))

        self.statusItem?.menu = menu
        let title = self.payload.currentTitle?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.statusItem?.button?.toolTip = title?.isEmpty == false ? "Codex Switch - \(title!)" : "Codex Switch"
    }

    private func makeQuotaCardItem(labels: TrayLabels) -> NSMenuItem {
        let card = TrayQuotaCard(payload: self.payload, labels: labels)
            .frame(width: 326)
            .fixedSize(horizontal: false, vertical: true)

        let hosting = NSHostingView(rootView: card)
        hosting.frame = NSRect(x: 0, y: 0, width: 326, height: self.payload.currentQuota == nil ? 92 : 170)

        let item = NSMenuItem()
        item.view = hosting
        item.isEnabled = false
        item.representedObject = "quota-card"
        return item
    }

    private func makeSwitchMenuItem(labels: TrayLabels) -> NSMenuItem {
        let item = NSMenuItem(title: labels.switchAccounts, action: nil, keyEquivalent: "")
        let submenu = NSMenu(title: labels.switchAccounts)
        submenu.autoenablesItems = false

        let profiles = self.payload.profiles ?? []
        if profiles.isEmpty {
            let empty = NSMenuItem(title: labels.noAccount, action: nil, keyEquivalent: "")
            empty.isEnabled = false
            submenu.addItem(empty)
        } else {
            for profile in profiles {
                let five = Self.percentText(profile.quota.fiveHour.remainingPercent)
                let weekly = Self.percentText(profile.quota.weekly.remainingPercent)
                let row = NSMenuItem(
                    title: "\(profile.menuTitle)   5h \(five) / 7d \(weekly)",
                    action: #selector(self.switchProfile(_:)),
                    keyEquivalent: ""
                )
                row.target = self
                row.representedObject = profile.folderName
                row.toolTip = profile.nickname.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? profile.displayTitle : profile.nickname
                row.isEnabled = true
                submenu.addItem(row)
            }
        }

        item.submenu = submenu
        return item
    }

    private func actionItem(title: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.isEnabled = true
        return item
    }

    @objc private func showMain(_ sender: NSMenuItem) {
        self.send(event: "tray_show_main")
    }

    @objc private func openSettings(_ sender: NSMenuItem) {
        self.send(event: "tray_settings")
    }

    @objc private func openAbout(_ sender: NSMenuItem) {
        self.send(event: "tray_about")
    }

    @objc private func quit(_ sender: NSMenuItem) {
        self.send(event: "tray_quit")
    }

    @objc private func switchProfile(_ sender: NSMenuItem) {
        guard let folderName = sender.representedObject as? String else { return }
        self.send(event: "tray_switch_profile", payload: folderName)
    }

    private func send(event: String, payload: String? = nil) {
        guard let callback else { return }
        event.withCString { eventPointer in
            if let payload {
                payload.withCString { payloadPointer in
                    callback(eventPointer, payloadPointer)
                }
            } else {
                callback(eventPointer, nil)
            }
        }
    }

    private static func percentText(_ value: UInt8?) -> String {
        guard let value else { return "--" }
        return "\(min(value, 100))%"
    }

    private static func makeTemplateIcon(bytes: [UInt8]) -> NSImage? {
        let width = 32
        let height = 32
        let data = Data(bytes)
        guard
            let provider = CGDataProvider(data: data as CFData),
            let image = CGImage(
                width: width,
                height: height,
                bitsPerComponent: 8,
                bitsPerPixel: 32,
                bytesPerRow: width * 4,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
                provider: provider,
                decode: nil,
                shouldInterpolate: true,
                intent: .defaultIntent
            )
        else {
            return nil
        }

        let nsImage = NSImage(cgImage: image, size: NSSize(width: 22, height: 22))
        nsImage.isTemplate = true
        return nsImage
    }
}

private struct TrayQuotaCard: View {
    let payload: TrayPayload
    let labels: TrayLabels

    private var title: String {
        let current = payload.currentTitle?.trimmingCharacters(in: .whitespacesAndNewlines)
        return current?.isEmpty == false ? current! : labels.noAccount
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(labels.current)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.secondary)
                Spacer(minLength: 10)
            }

            Text(title)
                .font(.system(size: 14, weight: .semibold))
                .lineLimit(1)
                .truncationMode(.middle)

            if let quota = payload.currentQuota {
                VStack(alignment: .leading, spacing: 12) {
                    QuotaProgressRow(title: labels.fiveHour, window: quota.fiveHour, labels: labels)
                    QuotaProgressRow(title: labels.weekly, window: quota.weekly, labels: labels)
                }
            } else {
                Text(labels.noQuota)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .padding(.top, 2)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .background(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(.regularMaterial)
                .overlay(
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .stroke(Color.primary.opacity(0.08), lineWidth: 1)
                )
        )
        .padding(.horizontal, 6)
        .padding(.vertical, 6)
    }
}

private struct QuotaProgressRow: View {
    let title: String
    let window: QuotaWindow
    let labels: TrayLabels

    private var left: Int {
        Int(min(window.remainingPercent ?? 0, 100))
    }

    private var used: Int {
        max(0, 100 - left)
    }

    private var tint: Color {
        if left > 60 { return .green }
        if left >= 20 { return .orange }
        return .red
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Text(title)
                    .font(.system(size: 18, weight: .bold, design: .rounded))
                    .frame(width: 36, alignment: .leading)
                VStack(alignment: .leading, spacing: 5) {
                    ProgressBar(percent: Double(used), tint: tint)
                    HStack(spacing: 8) {
                        Text("\(labels.used) \(String(format: "%.1f", Double(used)))%")
                        Spacer(minLength: 4)
                        Text("\(labels.left) \(String(format: "%.1f", Double(left)))%")
                            .foregroundStyle(tint)
                        Spacer(minLength: 4)
                        Text("\(labels.resets) \(resetText)")
                    }
                    .font(.system(size: 10.5, weight: .semibold))
                    .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var resetText: String {
        let value = window.refreshAt?.trimmingCharacters(in: .whitespacesAndNewlines)
        return value?.isEmpty == false ? value! : "--"
    }
}

private struct ProgressBar: View {
    let percent: Double
    let tint: Color

    private var clamped: Double {
        min(100, max(0, percent))
    }

    var body: some View {
        GeometryReader { proxy in
            ZStack(alignment: .leading) {
                Capsule()
                    .fill(Color.primary.opacity(0.12))
                Capsule()
                    .fill(tint.opacity(0.82))
                    .frame(width: proxy.size.width * clamped / 100)
            }
        }
        .frame(height: 7)
    }
}

@_cdecl("codex_switch_native_tray_install")
public func codexSwitchNativeTrayInstall(
    _ iconBytes: UnsafePointer<UInt8>?,
    _ iconLength: Int,
    _ callback: TrayCallback?
) {
    DispatchQueue.main.async {
        CodexSwitchNativeTrayController.shared.install(
            iconBytes: iconBytes,
            iconLength: iconLength,
            callback: callback
        )
    }
}

@_cdecl("codex_switch_native_tray_sync")
public func codexSwitchNativeTraySync(_ json: UnsafePointer<CChar>?) {
    guard let json else { return }
    let value = String(cString: json)
    DispatchQueue.main.async {
        CodexSwitchNativeTrayController.shared.sync(json: value)
    }
}
