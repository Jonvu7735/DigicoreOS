/** Map a tier/status value to a coloured badge class (falls back to a neutral pill). */
export function badgeClass(value?: string | null): string {
  switch ((value ?? "").toUpperCase()) {
    case "GOLD":
      return "badge badge-gold";
    case "BRONZE":
      return "badge badge-orange";
    case "SILVER":
    case "DRAFT":
      return "badge badge-gray";
    case "BOOKED":
      return "badge badge-blue";
    case "DISPATCHED":
      return "badge badge-green";
    case "CANCELLED":
      return "badge badge-red";
    default:
      return "pill";
  }
}
