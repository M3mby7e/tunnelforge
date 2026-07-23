import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

export const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
  {
    variants: {
      variant: {
        neutral: "bg-muted text-muted-foreground",
        outline: "border text-muted-foreground",
        brand:
          "bg-[color-mix(in_oklch,var(--brand)_18%,transparent)] text-brand",
        success:
          "bg-[color-mix(in_oklch,var(--success)_18%,transparent)] text-success",
        warning:
          "bg-[color-mix(in_oklch,var(--warning)_22%,transparent)] text-warning",
        danger:
          "bg-[color-mix(in_oklch,var(--destructive)_18%,transparent)] text-destructive",
      },
    },
    defaultVariants: {
      variant: "neutral",
    },
  },
);

export interface BadgeProps
  extends
    React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}
