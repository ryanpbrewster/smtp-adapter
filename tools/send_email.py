#!/usr/bin/env python3

import argparse
import email
import smtplib

parser = argparse.ArgumentParser(description="Interact with an SMTP server",
                                 add_help=False)
parser.add_argument("files", nargs="*", type=argparse.FileType("r"))
parser.add_argument("--host", default="127.0.0.1")
parser.add_argument("--port", type=int, default=3025)
parser.add_argument("--help", action="help")
args = parser.parse_args()

with smtplib.SMTP(host=args.host, port=args.port) as smtp:
    smtp.set_debuglevel(2)
    for filename in args.files:
        print(f"sending {filename}...")
        smtp.send_message(email.message_from_file(filename))
