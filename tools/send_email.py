#!/usr/bin/env python3

import smtplib
import argparse
import email

parser = argparse.ArgumentParser(
    description="Send an email using SMTP.", add_help=False
)
parser.add_argument(
    "eml_files",
    nargs="*",
    type=argparse.FileType("r"),
    help='EML file of message to send, e.g. "./kotlin/shortwave/email/reply/testdata/plain_text_reply_emails/gmail.eml"',
)
parser.add_argument("-h", "--host", default="127.0.0.1")
parser.add_argument("-p", "--port", type=int, default=3025)
parser.add_argument("--help", action="help")
args = parser.parse_args()

with smtplib.SMTP(host=args.host, port=args.port) as smtp:
    smtp.set_debuglevel(2)
    for eml_file in args.eml_files:
        print(f"sending {eml_file}...")
        msg = email.message_from_file(eml_file)
        # smtp.send_message(msg, from_addr=args.user, to_addrs=args.to)
        smtp.send_message(msg)
