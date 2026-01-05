const std = @import("std");
const version_mod = @import("version.zig");

pub const AppScriptConfig = struct {
    script: []const u8,
};

pub const App = struct {
    name: []const u8,
    description: ?[]const u8 = null,
    local: AppScriptConfig,
    remote: AppScriptConfig,
    update: AppScriptConfig,
    compare_mode: version_mod.CompareMode = .string,

    pub fn deinit(self: *App, allocator: std.mem.Allocator) void {
        allocator.free(self.name);
        if (self.description) |desc| allocator.free(desc);
        allocator.free(self.local.script);
        allocator.free(self.remote.script);
        allocator.free(self.update.script);
    }
};

pub const Config = struct {
    apps: []App,
    allocator: std.mem.Allocator,

    pub fn deinit(self: *Config) void {
        for (self.apps) |*app| {
            app.deinit(self.allocator);
        }
        self.allocator.free(self.apps);
    }

    pub fn validate(self: *Config) !void {
        for (self.apps) |app| {
            if (app.name.len == 0) {
                return error.InvalidConfig;
            }

            // Check scripts exist and are executable
            try validateScript(app.local.script);
            try validateScript(app.remote.script);
            try validateScript(app.update.script);
        }
    }

    fn validateScript(path: []const u8) !void {
        const file = std.fs.openFileAbsolute(path, .{}) catch |err| {
            return err;
        };
        defer file.close();

        const stat = try file.stat();
        if (stat.kind != .file) {
            return error.NotAFile;
        }

        // Check if executable (Unix permissions)
        const mode = stat.mode;
        const is_executable = (mode & 0o111) != 0;
        if (!is_executable) {
            return error.NotExecutable;
        }
    }

    pub fn loadFromFile(allocator: std.mem.Allocator, path: []const u8) !Config {
        const file = try std.fs.openFileAbsolute(path, .{});
        defer file.close();

        const content = try file.readToEndAlloc(allocator, 10 * 1024 * 1024);
        defer allocator.free(content);

        return try parseToml(allocator, content);
    }

    fn parseToml(allocator: std.mem.Allocator, content: []const u8) !Config {
        var apps: std.ArrayList(App) = .empty;
        errdefer {
            for (apps.items) |*app| {
                app.deinit(allocator);
            }
            apps.deinit(allocator);
        }

        var lines = std.mem.splitScalar(u8, content, '\n');
        var current_app: ?App = null;
        var in_local = false;
        var in_remote = false;
        var in_update = false;

        while (lines.next()) |line| {
            const trimmed = std.mem.trim(u8, line, &std.ascii.whitespace);

            if (trimmed.len == 0 or trimmed[0] == '#') continue;

            if (std.mem.eql(u8, trimmed, "[[app]]")) {
                if (current_app) |app| {
                    try apps.append(allocator, app);
                }
                current_app = App{
                    .name = "",
                    .description = null,
                    .local = .{ .script = "" },
                    .remote = .{ .script = "" },
                    .update = .{ .script = "" },
                    .compare_mode = .string,
                };
                in_local = false;
                in_remote = false;
                in_update = false;
                continue;
            }

            if (std.mem.eql(u8, trimmed, "[app.local]")) {
                in_local = true;
                in_remote = false;
                in_update = false;
                continue;
            }

            if (std.mem.eql(u8, trimmed, "[app.remote]")) {
                in_local = false;
                in_remote = true;
                in_update = false;
                continue;
            }

            if (std.mem.eql(u8, trimmed, "[app.update]")) {
                in_local = false;
                in_remote = false;
                in_update = true;
                continue;
            }

            if (current_app) |*app| {
                if (std.mem.indexOf(u8, trimmed, "name = ")) |_| {
                    const value = try parseStringValue(allocator, trimmed);
                    app.name = value;
                } else if (std.mem.indexOf(u8, trimmed, "description = ")) |_| {
                    const value = try parseStringValue(allocator, trimmed);
                    app.description = value;
                } else if (std.mem.indexOf(u8, trimmed, "compare = ")) |_| {
                    const value = try parseStringValue(allocator, trimmed);
                    defer allocator.free(value);
                    app.compare_mode = version_mod.CompareMode.fromString(value) catch .string;
                } else if (std.mem.indexOf(u8, trimmed, "script = ")) |_| {
                    const value = try parseStringValue(allocator, trimmed);
                    if (in_local) {
                        app.local.script = value;
                    } else if (in_remote) {
                        app.remote.script = value;
                    } else if (in_update) {
                        app.update.script = value;
                    }
                }
            }
        }

        if (current_app) |app| {
            try apps.append(allocator, app);
        }

        return Config{
            .apps = try apps.toOwnedSlice(allocator),
            .allocator = allocator,
        };
    }

    fn parseStringValue(allocator: std.mem.Allocator, line: []const u8) ![]const u8 {
        if (std.mem.indexOf(u8, line, "\"")) |start_quote| {
            const after_first = line[start_quote + 1 ..];
            if (std.mem.indexOf(u8, after_first, "\"")) |end_quote| {
                const value = after_first[0..end_quote];
                return try allocator.dupe(u8, value);
            }
        }
        return error.InvalidToml;
    }
};

test "parse simple TOML" {
    const allocator = std.testing.allocator;

    const toml =
        \\[[app]]
        \\name = "dust"
        \\description = "du replacement"
        \\
        \\[app.local]
        \\script = "/tmp/local.sh"
        \\
        \\[app.remote]
        \\script = "/tmp/remote.sh"
        \\
        \\[app.update]
        \\script = "/tmp/update.sh"
    ;

    var config = try Config.parseToml(allocator, toml);
    defer config.deinit();

    try std.testing.expectEqual(@as(usize, 1), config.apps.len);
    try std.testing.expectEqualStrings("dust", config.apps[0].name);
    try std.testing.expectEqual(version_mod.CompareMode.string, config.apps[0].compare_mode);
}

test "parse TOML with semver compare mode" {
    const allocator = std.testing.allocator;

    const toml =
        \\[[app]]
        \\name = "test"
        \\compare = "semver"
        \\
        \\[app.local]
        \\script = "/tmp/local.sh"
        \\
        \\[app.remote]
        \\script = "/tmp/remote.sh"
        \\
        \\[app.update]
        \\script = "/tmp/update.sh"
    ;

    var config = try Config.parseToml(allocator, toml);
    defer config.deinit();

    try std.testing.expectEqual(@as(usize, 1), config.apps.len);
    try std.testing.expectEqual(version_mod.CompareMode.semver, config.apps[0].compare_mode);
}

test "parse TOML with multiple apps" {
    const allocator = std.testing.allocator;

    const toml =
        \\[[app]]
        \\name = "dust"
        \\
        \\[app.local]
        \\script = "/tmp/dust_local.sh"
        \\
        \\[app.remote]
        \\script = "/tmp/dust_remote.sh"
        \\
        \\[app.update]
        \\script = "/tmp/dust_update.sh"
        \\
        \\[[app]]
        \\name = "fd"
        \\description = "find alternative"
        \\
        \\[app.local]
        \\script = "/tmp/fd_local.sh"
        \\
        \\[app.remote]
        \\script = "/tmp/fd_remote.sh"
        \\
        \\[app.update]
        \\script = "/tmp/fd_update.sh"
    ;

    var config = try Config.parseToml(allocator, toml);
    defer config.deinit();

    try std.testing.expectEqual(@as(usize, 2), config.apps.len);
    try std.testing.expectEqualStrings("dust", config.apps[0].name);
    try std.testing.expectEqualStrings("fd", config.apps[1].name);
    try std.testing.expect(config.apps[1].description != null);
}

test "parse TOML with comments and empty lines" {
    const allocator = std.testing.allocator;

    const toml =
        \\# This is a comment
        \\
        \\[[app]]
        \\name = "test"
        \\# Another comment
        \\description = "test app"
        \\
        \\[app.local]
        \\script = "/tmp/local.sh"
        \\
        \\[app.remote]
        \\script = "/tmp/remote.sh"
        \\
        \\[app.update]
        \\script = "/tmp/update.sh"
    ;

    var config = try Config.parseToml(allocator, toml);
    defer config.deinit();

    try std.testing.expectEqual(@as(usize, 1), config.apps.len);
    try std.testing.expectEqualStrings("test", config.apps[0].name);
}
