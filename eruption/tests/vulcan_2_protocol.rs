/* SPDX-License-Identifier: GPL-3.0-or-later */

#[path = "../src/hwdevices/roccat_vulcan_2_protocol.rs"]
mod protocol;

use protocol::{encode_direct, DeviceInfo, LengthByteOrder, REPORT_COUNT, REPORT_LENGTH};

#[test]
fn actual_german_device_report_is_parsed() {
    let info = DeviceInfo::parse(&[0x09, 0x09, 0x6d, 0x65, 0x65, 0, 1, 0, 1]).unwrap();
    assert_eq!(info.firmware_major, 1);
    assert_eq!(info.firmware_minor, 9);
    assert_eq!(info.layout, 1);
    assert_eq!(info.length_byte_order(), LengthByteOrder::LittleEndian);
}

#[test]
fn newer_firmware_uses_reference_byte_order_rule() {
    let info = DeviceInfo::parse(&[9, 9, 116, 0, 0, 0, 1, 0, 0]).unwrap();
    assert_eq!(info.length_byte_order(), LengthByteOrder::BigEndian);
}

#[test]
fn malformed_information_reports_are_rejected() {
    for len in 0..9 {
        assert!(DeviceInfo::parse(&vec![9; len]).is_err());
    }
    assert!(DeviceInfo::parse(&[9; 10]).is_err());
    assert!(DeviceInfo::parse(&[8, 9, 109, 0, 0, 0, 1, 0, 0]).is_err());
    assert!(DeviceInfo::parse(&[9, 8, 109, 0, 0, 0, 1, 0, 0]).is_err());
}

#[test]
fn unknown_layout_is_preserved_for_caller_validation() {
    let info = DeviceInfo::parse(&[9, 9, 109, 0, 0, 0, 255, 0, 0]).unwrap();
    assert_eq!(info.layout, 255);
}

#[test]
fn black_frame_has_numbered_reports_and_zero_padding() {
    let reports = encode_direct(&[], LengthByteOrder::LittleEndian).unwrap();
    assert_eq!(reports.len(), REPORT_COUNT);
    for (index, report) in reports.iter().enumerate() {
        assert_eq!(report.len(), REPORT_LENGTH);
        assert_eq!(&report[..3], &[0, 0xa1, index as u8 + 1]);
        if index == 0 {
            assert_eq!(&report[3..5], &[0x8c, 0x01]);
        } else {
            assert_eq!(&report[3..5], &[0, 0]);
        }
        assert!(report[5..].iter().all(|&byte| byte == 0));
    }
}

#[test]
fn length_byte_order_is_explicit() {
    let reports = encode_direct(&[], LengthByteOrder::BigEndian).unwrap();
    assert_eq!(&reports[0][3..5], &[0x01, 0x8c]);
}

#[test]
fn rgb_addresses_cross_packet_boundaries_without_touching_headers() {
    let reports = encode_direct(
        &[
            (0, [1, 2, 3]),
            (19, [4, 5, 6]),
            (20, [7, 8, 9]),
            (129, [10, 11, 12]),
        ],
        LengthByteOrder::LittleEndian,
    )
    .unwrap();
    assert_eq!(&reports[0][5..8], &[1, 2, 3]);
    assert_eq!(&reports[0][62..65], &[4, 5, 6]);
    assert_eq!(&reports[1][5..8], &[7, 8, 9]);
    assert_eq!(&reports[6][32..35], &[10, 11, 12]);
    assert!(reports[6][35..].iter().all(|&byte| byte == 0));
}

#[test]
fn invalid_and_duplicate_addresses_are_rejected() {
    assert!(encode_direct(&[(130, [1, 2, 3])], LengthByteOrder::LittleEndian).is_err());
    assert!(encode_direct(&[(usize::MAX, [1, 2, 3])], LengthByteOrder::LittleEndian).is_err());
    assert!(encode_direct(
        &[(0, [1, 2, 3]), (0, [4, 5, 6])],
        LengthByteOrder::LittleEndian
    )
    .is_err());
}
