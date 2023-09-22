#!/usr/bin/env python3
#bodge

import argparse
from random import shuffle
from dataclasses import dataclass
import subprocess
from math import ceil

@dataclass
class Command:
    image_name: str
    size: int

    def get_command(self, width: int, height: int):
        print(self.size)
        s = "{}x{}".format(ceil(width / self.size), ceil(height / self.size))

        out = [
            'cargo', 'run', '--release', 
            '--', 
            '-Vvv', 
            '-i', str(self.size),
            self.image_name, 
            '-o' , s, 
            '--hold', '5',
        ]

        return out

commands = [
    Command("images/circuit-1-57x30.png", 14),
]

def main():
    parser = argparse.ArgumentParser(description='WFC demo mode')
    
    parser.add_argument('--slow', '-s', action='store_true', help='Run in slow mode')
    parser.add_argument('--debug', '-d', action='store_true', help='Entropy debug')
    parser.add_argument('--fullscreen', '-f', action='store_true', help='Full screen mode')
    
    parser.add_argument('--width', '-W', type=int, required=True, help='Width')
    parser.add_argument('--height', '-H', type=int, required=True, help='Height')

    args = parser.parse_args()

    while True:
        targets = [*commands]
        shuffle(targets)

        for target in targets:
            cmd = target.get_command(args.width, args.height)

            if args.slow:
                cmd.append('--slow')
            
            if args.debug:
                cmd.append('--debug')

            if args.fullscreen:
                cmd.append('-f')

            print(' '.join(cmd))
            subprocess.call(cmd)
            
if __name__ == '__main__':
    main()