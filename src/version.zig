const std = @import("std");

pub const Version = struct {
    major: u32,
    minor: u32,
    patch: u32,

    pub fn parse(str: []const u8) !Version {
        const trimmed = std.mem.trim(u8, str, &std.ascii.whitespace);
        
        // Remove leading 'v' if present
        const version_str = if (trimmed.len > 0 and trimmed[0] == 'v')
            trimmed[1..]
        else
            trimmed;

        var parts = std.mem.splitScalar(u8, version_str, '.');
        
        const major_str = parts.next() orelse return error.InvalidVersion;
        const minor_str = parts.next() orelse return error.InvalidVersion;
        const patch_str = parts.next() orelse return error.InvalidVersion;

        return Version{
            .major = try std.fmt.parseInt(u32, major_str, 10),
            .minor = try std.fmt.parseInt(u32, minor_str, 10),
            .patch = try std.fmt.parseInt(u32, patch_str, 10),
        };
    }

    pub fn compare(self: Version, other: Version) std.math.Order {
        if (self.major != other.major) {
            return std.math.order(self.major, other.major);
        }
        if (self.minor != other.minor) {
            return std.math.order(self.minor, other.minor);
        }
        return std.math.order(self.patch, other.patch);
    }

    pub fn lessThan(self: Version, other: Version) bool {
        return self.compare(other) == .lt;
    }

    pub fn equals(self: Version, other: Version) bool {
        return self.compare(other) == .eq;
    }
};

pub const CompareMode = enum {
    string,
    semver,

    pub fn fromString(str: []const u8) !CompareMode {
        if (std.mem.eql(u8, str, "string")) return .string;
        if (std.mem.eql(u8, str, "semver")) return .semver;
        return error.InvalidCompareMode;
    }
};

test "parse simple semver" {
    const v = try Version.parse("1.2.3");
    try std.testing.expectEqual(@as(u32, 1), v.major);
    try std.testing.expectEqual(@as(u32, 2), v.minor);
    try std.testing.expectEqual(@as(u32, 3), v.patch);
}

test "parse semver with v prefix" {
    const v = try Version.parse("v2.0.1");
    try std.testing.expectEqual(@as(u32, 2), v.major);
    try std.testing.expectEqual(@as(u32, 0), v.minor);
    try std.testing.expectEqual(@as(u32, 1), v.patch);
}

test "parse semver with whitespace" {
    const v = try Version.parse("  3.4.5\n");
    try std.testing.expectEqual(@as(u32, 3), v.major);
    try std.testing.expectEqual(@as(u32, 4), v.minor);
    try std.testing.expectEqual(@as(u32, 5), v.patch);
}

test "version comparison" {
    const v1 = try Version.parse("1.2.3");
    const v2 = try Version.parse("1.2.4");
    const v3 = try Version.parse("1.3.0");
    const v4 = try Version.parse("2.0.0");
    const v5 = try Version.parse("1.2.3");

    try std.testing.expect(v1.lessThan(v2));
    try std.testing.expect(v1.lessThan(v3));
    try std.testing.expect(v1.lessThan(v4));
    try std.testing.expect(v1.equals(v5));
    try std.testing.expect(!v2.lessThan(v1));
}

test "compare mode parsing" {
    try std.testing.expectEqual(CompareMode.string, try CompareMode.fromString("string"));
    try std.testing.expectEqual(CompareMode.semver, try CompareMode.fromString("semver"));
    try std.testing.expectError(error.InvalidCompareMode, CompareMode.fromString("invalid"));
}
