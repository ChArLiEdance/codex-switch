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
    let accountQuota: String

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
                noQuota: "暂无额度数据",
                accountQuota: "当前额度"
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
            noQuota: "No quota data",
            accountQuota: "Current Quota"
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
        item.button?.imageScaling = .scaleProportionallyDown
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
        menu.minimumWidth = 368

        let root = TrayMenuRootView(
            payload: self.payload,
            labels: labels,
            onAction: { [weak self] event, payload in
                self?.perform(event: event, payload: payload)
            }
        )
        .frame(width: 368)
        .fixedSize(horizontal: false, vertical: true)

        let hosting = NSHostingView(rootView: root)
        hosting.wantsLayer = true
        hosting.layer?.backgroundColor = NSColor.clear.cgColor
        hosting.frame = NSRect(
            x: 0,
            y: 0,
            width: 368,
            height: Self.menuHeight(for: self.payload)
        )

        let item = NSMenuItem()
        item.view = hosting
        item.isEnabled = true
        item.representedObject = "swiftui-root-menu"
        menu.addItem(item)

        self.statusItem?.menu = menu
        let title = self.payload.currentTitle?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.statusItem?.button?.toolTip = title?.isEmpty == false ? "Codex Switch - \(title!)" : "Codex Switch"
    }

    private func perform(event: String, payload: String? = nil) {
        self.statusItem?.menu?.cancelTracking()
        self.send(event: event, payload: payload)
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

    private static func menuHeight(for payload: TrayPayload) -> CGFloat {
        let quotaHeight: CGFloat = payload.currentQuota == nil ? 112 : 198
        let profileCount = CGFloat(max(payload.profiles?.count ?? 0, 1))
        let profileHeight = min(profileCount * 42, 190)
        return 34 + quotaHeight + 46 + 34 + profileHeight + 118
    }

    private static func makeTemplateIcon(bytes: [UInt8]) -> NSImage? {
        guard let nsImage = NSImage(data: Data(bytes)) else {
            return nil
        }
        nsImage.size = NSSize(width: 18, height: 18)
        nsImage.isTemplate = true
        return nsImage
    }
}

private struct TrayMenuRootView: View {
    let payload: TrayPayload
    let labels: TrayLabels
    let onAction: (String, String?) -> Void

    private var profiles: [TrayProfileEntry] {
        payload.profiles ?? []
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            TrayMenuHeader(payload: payload, labels: labels)

            TrayQuotaCard(payload: payload, labels: labels)

            TrayMenuActionRow(title: labels.show, systemImage: "macwindow", prominent: true) {
                onAction("tray_show_main", nil)
            }

            VStack(alignment: .leading, spacing: 6) {
                Text(labels.switchAccounts)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 4)

                if profiles.isEmpty {
                    Text(labels.noAccount)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(.secondary)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 8)
                        .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                } else {
                    ScrollView {
                        VStack(spacing: 6) {
                            ForEach(profiles) { profile in
                                TrayProfileRow(profile: profile) {
                                    onAction("tray_switch_profile", profile.folderName)
                                }
                            }
                        }
                    }
                    .frame(maxHeight: 190)
                }
            }

            VStack(spacing: 6) {
                TrayMenuActionRow(title: labels.settings, systemImage: "gearshape") {
                    onAction("tray_settings", nil)
                }
                TrayMenuActionRow(title: labels.about, systemImage: "info.circle") {
                    onAction("tray_about", nil)
                }
                TrayMenuActionRow(title: labels.quit, systemImage: "power", destructive: true) {
                    onAction("tray_quit", nil)
                }
            }
        }
        .padding(10)
    }
}

private struct TrayMenuHeader: View {
    let payload: TrayPayload
    let labels: TrayLabels

    private var currentTitle: String {
        let current = payload.currentTitle?.trimmingCharacters(in: .whitespacesAndNewlines)
        return current?.isEmpty == false ? current! : labels.noAccount
    }

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "terminal.fill")
                .font(.system(size: 16, weight: .bold))
                .foregroundStyle(.blue)
                .frame(width: 30, height: 30)
                .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 10, style: .continuous))

            VStack(alignment: .leading, spacing: 2) {
                Text("Codex Switch")
                    .font(.system(size: 13, weight: .semibold))
                Text(currentTitle)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer(minLength: 0)
        }
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
                Text(labels.accountQuota)
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

private struct TrayMenuActionRow: View {
    let title: String
    let systemImage: String
    var prominent = false
    var destructive = false
    let action: () -> Void

    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 9) {
                Image(systemName: systemImage)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(iconColor)
                    .frame(width: 18)
                Text(title)
                    .font(.system(size: 12.5, weight: .semibold))
                    .foregroundStyle(textColor)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .background(rowBackground)
        }
        .buttonStyle(.plain)
        .onHover { hovered = $0 }
    }

    private var rowBackground: some View {
        RoundedRectangle(cornerRadius: 10, style: .continuous)
            .fill(backgroundMaterial)
            .overlay(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .stroke(Color.primary.opacity(hovered || prominent ? 0.10 : 0.06), lineWidth: 1)
            )
    }

    private var backgroundMaterial: Material {
        prominent || hovered ? .regularMaterial : .thinMaterial
    }

    private var iconColor: Color {
        if destructive { return .red }
        if prominent { return .blue }
        return .primary.opacity(0.76)
    }

    private var textColor: Color {
        if destructive { return .red }
        return .primary
    }
}

private struct TrayProfileRow: View {
    let profile: TrayProfileEntry
    let action: () -> Void

    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: "person.crop.circle")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(.blue)
                    .frame(width: 20)

                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 6) {
                        Text(profile.menuTitle)
                            .font(.system(size: 12.5, weight: .semibold))
                            .lineLimit(1)
                            .truncationMode(.middle)
                        if let plan = profile.planName?.trimmingCharacters(in: .whitespacesAndNewlines), !plan.isEmpty {
                            Text(plan)
                                .font(.system(size: 9.5, weight: .bold))
                                .foregroundStyle(.secondary)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(.thinMaterial, in: Capsule())
                        }
                    }

                    Text("5h \(percentText(profile.quota.fiveHour.remainingPercent))  ·  7d \(percentText(profile.quota.weekly.remainingPercent))")
                        .font(.system(size: 10.5, weight: .semibold))
                        .foregroundStyle(.secondary)
                }

                Spacer(minLength: 0)
                Image(systemName: "chevron.right")
                    .font(.system(size: 10, weight: .bold))
                    .foregroundStyle(.tertiary)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .contentShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .background(
                RoundedRectangle(cornerRadius: 10, style: .continuous)
                    .fill(hovered ? .regularMaterial : .thinMaterial)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10, style: .continuous)
                            .stroke(Color.primary.opacity(hovered ? 0.10 : 0.06), lineWidth: 1)
                    )
            )
        }
        .buttonStyle(.plain)
        .onHover { hovered = $0 }
        .help(profile.nickname.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? profile.displayTitle : profile.nickname)
    }

    private func percentText(_ value: UInt8?) -> String {
        guard let value else { return "--" }
        return "\(min(value, 100))%"
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
