const std = @import("std");

pub const ScriptResult = struct {
    stdout: []u8,
    exit_code: u8,
    allocator: std.mem.Allocator,

    pub fn deinit(self: *ScriptResult) void {
        self.allocator.free(self.stdout);
    }
};

pub fn runScript(
    allocator: std.mem.Allocator,
    script_path: []const u8,
) !ScriptResult {
    const argv = [_][]const u8{ "sh", "-c", script_path };

    var child = std.process.Child.init(&argv, allocator);
    child.stdout_behavior = .Pipe;
    child.stderr_behavior = .Inherit;

    try child.spawn();

    const stdout = try child.stdout.?.readToEndAlloc(allocator, 1024 * 1024);
    errdefer allocator.free(stdout);

    const term = try child.wait();

    const exit_code: u8 = switch (term) {
        .Exited => |code| @intCast(code),
        else => 1,
    };

    return ScriptResult{
        .stdout = stdout,
        .exit_code = exit_code,
        .allocator = allocator,
    };
}

pub fn trimVersion(version: []const u8) []const u8 {
    return std.mem.trim(u8, version, &std.ascii.whitespace);
}

test "trimVersion removes whitespace" {
    try std.testing.expectEqualStrings("1.2.3", trimVersion("1.2.3\n"));
    try std.testing.expectEqualStrings("abc123", trimVersion("  abc123  "));
}
