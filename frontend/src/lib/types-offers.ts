// Offer payloads (src/view/offers.rs), the legacy OfferResource shapes.
import type { ModuleDetail } from './types';

export interface OfferParticipant {
  id: number;
  name: string;
}

export interface OfferListItem {
  id: number;
  sender: OfferParticipant;
  receiver: OfferParticipant;
  module: { id: number; type_id: number; type_name: string };
  price: number;
  latest_message: { content: string; sender_id: number; created_at: string };
  is_read: boolean;
  created_at: string;
}

export interface OfferMessage {
  id: number;
  sender: OfferParticipant;
  content: string;
  created_at: string;
  mine: boolean;
}

export interface OfferThread {
  id: number;
  sender: OfferParticipant;
  receiver: OfferParticipant;
  price: number;
  own_character_id: number;
  left_by_sender: boolean;
  left_by_receiver: boolean;
  module: ModuleDetail | null;
  messages: OfferMessage[];
}

/** One entry of /api/offers/sent: the buyer already has this module in
 * an active thread, so cards swap Make offer for Go to offer. */
export interface SentOffer {
  id: number;
  module_id: number;
}
