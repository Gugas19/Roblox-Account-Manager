export function Flag({ code, className = '' }: { code: string; className?: string }) {
  return <img src={`./flags/${code}.png`} alt="" className={`h-3.5 w-5 rounded-[2px] object-cover shadow-sm ${className}`} />
}
