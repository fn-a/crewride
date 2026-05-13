/** Wrapper for unplugin-icons raw SVG strings */
function Icon({ raw, className }: { raw: string; className?: string }) {
    return <span className={className} dangerouslySetInnerHTML={{ __html: raw }} />;
}

export { Icon };
