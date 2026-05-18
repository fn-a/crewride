import * as React from 'react';
import { cn } from '@/utils/style';
import type { JSX } from "react";
import { Button } from '@/components/button';
import { MoonIcon, SunIcon } from 'lucide-react';

interface HeaderProps {
    title?: string | JSX.Element;
    className?: string;
    children?: React.ReactNode;
}

export function Header({ title, className, children }: HeaderProps) {
    return (
        <header
            className={cn(
                'flex h-12 shrink-0 items-center justify-between border-b px-4 box-content',
                className,
            )}
        >
            <div className="flex items-center gap-2 min-w-0">
                {title && (
                    <h1 className="truncate text-sm font-medium">{title}</h1>
                )}
            </div>
            <div className="flex items-center gap-1">{children}</div>
        </header>
    );
}

interface ThemeProps {
    className?: string;
}

export function Theme({ className }: ThemeProps) {
    const [dark, setDark] = React.useState(() => {
        if (typeof document === 'undefined') return false;
        return document.documentElement.classList.contains('dark');
    });

    const toggleDark = React.useCallback(() => {
        const next = !dark;
        setDark(next);
        document.documentElement.classList.toggle('dark', next);
        try {
            localStorage.setItem('crewride-theme', next ? 'dark' : 'light');
        } catch {
            /* noop */
        }
    }, [dark]);

    return (
        <Button
            variant="ghost"
            size="sm"
            onClick={toggleDark}
            className={className}
            type="button"
        >
            {dark ? (
                <SunIcon className="size-4" />
            ) : (
                <MoonIcon className="size-4" />
            )}
        </Button>
    );
}